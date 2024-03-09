use crate::{Instance, MAX_VELOCITY};
use aftgraphs::{block_on, spawn, Handle};
use async_std::{
    channel::{bounded, Receiver, Sender, TryRecvError},
    sync::Mutex,
};
use bacon_sci::{
    ivp::{Derivative, Euler, IVPError, IVPSolver, UserError},
    prelude::*,
};
use crossbeam::{
    deque::Injector,
    sync::{Parker, Unparker},
};
use rand::{distributions::Uniform, prelude::*, thread_rng};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub struct Physics {
    deque: Arc<Injector<(f32, BVector<f32, Dyn>)>>,
    scratchpad: Vec<(f32, BVector<f32, Dyn>)>,
    time: f32,
    radius: f32,
    aspect_ratio: f32,
    num_particles: usize,
    reset_tx: Sender<PhysicsMessage>,
    response: Receiver<bool>,
    request: Unparker,
    lock: Arc<Mutex<bool>>,
    _handle: Handle,
}

#[derive(Clone)]
struct PhysicsData {
    radius: f32,
    aspect_ratio: f32,
    velocities: Rc<RefCell<BVector<f32, Dyn>>>,
}

enum PhysicsMessage {
    /// time, radius, aspect_ratio
    Reset(f32, f32, f32),
    Spawn(usize),
    Pop(usize),
}

struct PhysicsThread {
    display: bool,
    deque: Arc<Injector<(f32, BVector<f32, Dyn>)>>,
    time: f32,
    radius: f32,
    aspect_ratio: f32,
    reset: Receiver<PhysicsMessage>,
    response: Sender<bool>,
    request: Parker,
    lock: Arc<Mutex<bool>>,
}

fn particle_derivative(
    _t: f32,
    y: &[f32],
    data: &mut PhysicsData,
) -> Result<BVector<f32, Dyn>, UserError> {
    let mut derivative = BVector::from_element_generic(Dyn(y.len()), U1, 0f32);
    let mut velocities = data.velocities.borrow_mut();

    for (particle_idx, state) in y.chunks_exact(2).enumerate() {
        let velocity = &mut velocities.as_mut_slice()[particle_idx * 2..(particle_idx + 1) * 2];

        if state[0] <= -1.0 + data.radius || state[0] >= 1.0 - data.radius {
            velocity[0] *= -1.0;
        }

        if state[1] <= -1.0 + data.radius * data.aspect_ratio
            || state[1] >= 1.0 - data.radius * data.aspect_ratio
        {
            velocity[1] *= -1.0;
        }

        derivative[particle_idx * 2] = velocity[0];
        derivative[particle_idx * 2 + 1] = velocity[1];
    }
    Ok(derivative)
}

impl PhysicsThread {
    pub async fn create(self) -> Handle {
        spawn(move || {
            let mut thread = self;
            block_on(async move {
                thread
                    .thread()
                    .await
                    .expect("aftgraphs::particles::physics::PhysicsThread: failed");
            });
        })
        .await
        .expect("aftgraphs::particles::physics::PhysicsThread: failed to spawn")
    }

    pub async fn thread(&mut self) -> Result<(), IVPError> {
        let mut num_particles = 0;
        let velocities = Rc::new(RefCell::new(BVector::from_element_generic(
            Dyn(num_particles * 2),
            U1::name(),
            0.0,
        )));

        let mut rng = thread_rng();
        let distribution = Uniform::new_inclusive(-1.0, 1.0);
        let velocity_distribution = Uniform::new_inclusive(0.0, MAX_VELOCITY);
        let angle_distribution = Uniform::new(0.0, std::f32::consts::TAU);
        let mut new_particles = vec![];

        let data = PhysicsData {
            radius: self.radius,
            aspect_ratio: self.aspect_ratio,
            velocities: velocities.clone(),
        };

        let mut solver = Euler::new_dyn(num_particles * 2)?
            .with_tolerance(1e-2)?
            .with_minimum_dt(0.1)?
            .with_maximum_dt(0.1)?
            .with_initial_time(self.time)?
            .with_ending_time(f32::INFINITY)?
            .with_initial_conditions(BVector::from_element_generic(Dyn(0), U1::name(), 0.0))?
            .with_derivative(
                Box::new(particle_derivative) as Box<dyn Derivative<f32, Dyn, PhysicsData>>
            )
            .solve(data)?;

        loop {
            let len = {
                let mut lock = self.lock.lock().await;
                if *lock {
                    *lock = false;
                    continue;
                }

                let next = solver.next().ok_or(IVPError::TimeEndOOB)??;
                self.time = next.0;
                self.deque.push(next);
                self.deque.len()
            };

            match self.reset.try_recv() {
                Ok(reset) => {
                    let (start_time, mut start_state) = {
                        let s = self.deque.steal();
                        if let Some(s) = s.success() {
                            s
                        } else {
                            (
                                self.time,
                                BVector::from_element_generic(Dyn(num_particles * 2), U1, 0f32),
                            )
                        }
                    };

                    match reset {
                        PhysicsMessage::Reset(time, radius, aspect_ratio) => {
                            self.time = time;
                            self.radius = radius;
                            self.aspect_ratio = aspect_ratio;
                        }
                        PhysicsMessage::Pop(num) => {
                            if num > num_particles {
                                num_particles = 0;
                            } else {
                                num_particles -= num;
                            }

                            let v = velocities.as_ref();
                            let mut v = v.borrow_mut();
                            let iter = v.into_iter().cloned().take(num_particles * 2);
                            *v = BVector::from_iterator_generic(Dyn(num_particles * 2), U1, iter);

                            let iter = start_state.into_iter().cloned().take(num_particles * 2);
                            start_state =
                                BVector::from_iterator_generic(Dyn(num_particles * 2), U1, iter);
                        }
                        PhysicsMessage::Spawn(num) => {
                            let mut idx = 0;
                            let mut failed_circles = 0;
                            new_particles.clear();

                            while idx < num && failed_circles < 50 {
                                let x = rng.sample(distribution);
                                let y = rng.sample(distribution);

                                let new_velocity = rng.sample(velocity_distribution);
                                let angle = rng.sample(angle_distribution);
                                let new_velocity =
                                    (new_velocity * angle.cos(), new_velocity * angle.sin());

                                if x <= -1.0 + self.radius || x >= 1.0 - self.radius {
                                    failed_circles += 1;
                                    continue;
                                }

                                if y <= -1.0 + self.radius * self.aspect_ratio
                                    || y >= 1.0 - self.radius * self.aspect_ratio
                                {
                                    failed_circles += 1;
                                    continue;
                                }

                                for circle in start_state.as_slice().chunks(2) {
                                    if (circle[0] - x).powi(2)
                                        + ((circle[1] - y) / self.aspect_ratio).powi(2)
                                        <= 4.0 * self.radius.powi(2)
                                    {
                                        failed_circles += 1;
                                        continue;
                                    }
                                }

                                new_particles.push(((x, y), new_velocity));
                                failed_circles = 0;
                                idx += 1;
                            }

                            if failed_circles == 50 {
                                self.response.send(false).await.expect("aftgraphs::particles::physics::PhysicsThread: Failed to send response");
                                continue;
                            }

                            num_particles += num;
                            let v = velocities.as_ref();
                            let mut v = v.borrow_mut();
                            let iter = v.into_iter().cloned().chain(
                                new_particles
                                    .iter()
                                    .flat_map(|&(_, v)| [v.0, v.1].into_iter()),
                            );
                            *v = BVector::from_iterator_generic(Dyn(num_particles * 2), U1, iter);

                            let iter = start_state.into_iter().cloned().chain(
                                new_particles
                                    .iter()
                                    .flat_map(|&(c, _)| [c.0, c.1].into_iter()),
                            );
                            start_state =
                                BVector::from_iterator_generic(Dyn(num_particles * 2), U1, iter);
                        }
                    }

                    self.time = start_time;

                    while self.deque.steal().success().is_some() {}

                    let data = PhysicsData {
                        radius: self.radius,
                        aspect_ratio: self.aspect_ratio,
                        velocities: velocities.clone(),
                    };

                    self.deque.push((self.time, start_state.clone()));
                    solver = Euler::new_dyn(num_particles * 2)?
                        .with_tolerance(1e-2)?
                        .with_minimum_dt(0.1)?
                        .with_maximum_dt(0.1)?
                        .with_initial_time(self.time)?
                        .with_ending_time(f32::INFINITY)?
                        .with_initial_conditions(start_state)?
                        .with_derivative(Box::new(particle_derivative)
                            as Box<dyn Derivative<f32, Dyn, PhysicsData>>)
                        .solve(data)?;

                    self.response.send(true).await.expect(
                        "aftgraphs::particles::physics::PhysicsThread: failed to send response",
                    );
                }
                Err(TryRecvError::Closed) => break Ok(()),
                _ => (),
            }

            if len > 2 && !self.display || len > 100 && self.display {
                self.request.park();
            }
        }
    }
}

impl Physics {
    pub async fn new(
        display: bool,
        time: f32,
        radius: f32,
        aspect_ratio: f32,
    ) -> Result<Self, IVPError> {
        let (tx, rx) = bounded(1);
        let (response_tx, response_rx) = bounded(1);

        let request = Parker::new();
        let request_unpark = request.unparker().clone();
        request_unpark.unpark();

        let reset = Arc::new(Mutex::new(false));

        let deque = Arc::new(Injector::new());

        let thread = PhysicsThread {
            display,
            deque: deque.clone(),
            time,
            radius,
            aspect_ratio,
            reset: rx,
            response: response_tx,
            request,
            lock: reset.clone(),
        };
        let handle = thread.create().await;

        Ok(Self {
            deque,
            scratchpad: vec![],
            time,
            radius,
            aspect_ratio,
            num_particles: 0,
            lock: reset,
            reset_tx: tx,
            response: response_rx,
            request: request_unpark,
            _handle: handle,
        })
    }

    pub fn len(&self) -> usize {
        self.num_particles
    }

    pub async fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        if aspect_ratio != self.aspect_ratio {
            self.aspect_ratio = aspect_ratio;
            self.reset().await;
        }
    }

    async fn reset(&mut self) {
        let mut lock = self.lock.lock().await;
        *lock = true;

        let mut state = BVector::from_element_generic(Dyn(self.num_particles * 2), U1, 0f32);
        let mut state_time = 0.0;
        loop {
            let s = self.deque.steal();
            if s.is_empty() {
                break;
            }

            if let Some(s) = s.success() {
                if s.0 <= self.time {
                    state = s.1;
                    state_time = s.0;
                }
            }
        }

        self.deque.push((state_time, state));
        self.time = state_time;
        drop(lock);

        self.reset_tx
            .send(PhysicsMessage::Reset(
                self.time,
                self.radius,
                self.aspect_ratio,
            ))
            .await
            .expect("aftgraphs::particles::Physics: failed to send reset message");
        self.request.unpark();
        self.response
            .recv()
            .await
            .expect("aftgraphs::particles::Physics: failed to receive response");
    }

    fn instances(&self, state: BVector<f32, Dyn>) -> Vec<Instance> {
        let mut instances = Vec::with_capacity(state.len() / 2);

        for particle in state.as_slice().chunks_exact(2) {
            instances.push(Instance {
                position: [particle[0], particle[1]],
                radius: self.radius,
                color: [1.0; 3],
            });
        }

        instances
    }

    pub async fn get_state(&mut self, t: f32) -> Vec<Instance> {
        self.scratchpad.clear();
        self.request.unpark();

        let (time_after, state_after) = loop {
            let s = self.deque.steal();

            if s.is_empty() {
                log::warn!("queue is empty");
                self.request.unpark();
            } else if let Some(s) = s.success() {
                if s.0 < t {
                    self.scratchpad.push(s);
                    continue;
                } else {
                    break s;
                }
            }
        };

        if self.scratchpad.len() > 10 {
            log::warn!("too many old states: {}", self.scratchpad.len());
        }

        if self.scratchpad.is_empty() {
            self.instances(state_after)
        } else {
            let (time_before, ref state_before) = self.scratchpad[self.scratchpad.len() - 1];
            let time_bracket = time_after - time_before;
            let interp_t = (t - time_before) / time_bracket;
            self.instances(state_before * interp_t + state_after * (1.0 - interp_t))
        }
    }

    pub async fn spawn(&mut self, num: usize) -> bool {
        self.reset_tx
            .send(PhysicsMessage::Spawn(num))
            .await
            .expect("aftgraphs::particles::Physics::push_circle: failed to send reset message");

        self.request.unpark();

        if self.response.recv().await.unwrap_or(false) {
            self.num_particles += num;
            true
        } else {
            false
        }
    }

    pub async fn pop(&mut self, num: usize) {
        self.reset_tx
            .send(PhysicsMessage::Pop(num))
            .await
            .expect("aftgraphs::particles::Physics::pop: failed to send reset message");
        self.request.unpark();
        self.response
            .recv()
            .await
            .expect("aftgraphs::particles::Physics::pop: failed to receive response");
        if self.num_particles > num {
            self.num_particles -= num;
        } else {
            self.num_particles = 0;
        }
    }
}
