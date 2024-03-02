use crate::Instance;
use bacon_sci::{
    ivp::{Derivative, Euler, EulerSolver, IVPError, IVPIterator, IVPSolver, UserError},
    prelude::*,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

type Solver = IVPIterator<
    Dyn,
    EulerSolver<'static, f32, Dyn, PhysicsData, Box<dyn Derivative<f32, Dyn, PhysicsData>>>,
>;

pub struct Physics {
    states: VecDeque<(f32, BVector<f32, Dyn>)>,
    velocities: Rc<RefCell<BVector<f32, Dyn>>>,
    time: f32,
    radius: f32,
    solver: Solver,
    aspect_ratio: f32,
}

#[derive(Clone)]
struct PhysicsData {
    radius: f32,
    aspect_ratio: f32,
    velocities: Rc<RefCell<BVector<f32, Dyn>>>,
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

impl Physics {
    pub fn new(time: f32, radius: f32, aspect_ratio: f32) -> Result<Self, IVPError> {
        let velocities = Rc::new(RefCell::new(BVector::from_element_generic(
            Dyn(0),
            U1::name(),
            0.0,
        )));

        let data = PhysicsData {
            radius,
            aspect_ratio,
            velocities: velocities.clone(),
        };

        let solver = Euler::new_dyn(0)?
            .with_tolerance(1e-4)?
            .with_minimum_dt(1e-4)?
            .with_maximum_dt(1.0 / 30.0)?
            .with_initial_time(time)?
            .with_ending_time(f32::INFINITY)?
            .with_initial_conditions(BVector::from_element_generic(Dyn(0), U1::name(), 0.0))?
            .with_derivative(
                Box::new(particle_derivative) as Box<dyn Derivative<f32, Dyn, PhysicsData>>
            )
            .solve(data)?;

        Ok(Self {
            states: VecDeque::new(),
            velocities,
            time,
            radius,
            solver,
            aspect_ratio,
        })
    }

    pub fn len(&self) -> usize {
        self.states.back().unwrap().1.len() / 2
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        if aspect_ratio != self.aspect_ratio {
            self.aspect_ratio = aspect_ratio;
            if let Some(state) = self.states.pop_back() {
                self.reset(state.1).unwrap();
            } else {
                self.reset(BVector::from_element_generic(Dyn(0), U1::name(), 0.0))
                    .unwrap();
            }
        }
    }

    fn reset(&mut self, state: BVector<f32, Dyn>) -> Result<(), IVPError> {
        let data = PhysicsData {
            radius: self.radius,
            aspect_ratio: self.aspect_ratio,
            velocities: self.velocities.clone(),
        };

        self.solver = Euler::new_dyn(state.len())?
            .with_tolerance(1e-4)?
            .with_minimum_dt(1e-4)?
            .with_maximum_dt(1.0 / 30.0)?
            .with_initial_time(self.time)?
            .with_ending_time(f32::INFINITY)?
            .with_initial_conditions(state.clone())?
            .with_derivative(
                Box::new(particle_derivative) as Box<dyn Derivative<f32, Dyn, PhysicsData>>
            )
            .solve(data)?;

        self.states.clear();
        self.states.push_front((self.time, state));
        Ok(())
    }

    pub fn simulate(&mut self, t: f32) {
        self.states.retain(|(st, _)| *st >= t);

        while self.time < t + 1.0 / 15.0 || self.states.len() < 10 {
            let next = self.solver.next().unwrap().unwrap();
            self.time = next.0;
            self.states.push_back(next);
        }
    }

    pub fn get_state(&self, t: f32) -> Vec<Instance> {
        let state_after = self
            .states
            .iter()
            .enumerate()
            .find_map(|(idx, state)| if state.0 >= t { Some(idx) } else { None })
            .unwrap();

        let state = if state_after > 0 {
            let time_bracket = self.states[state_after].0 - self.states[state_after - 1].0;
            let interp_t = (t - self.states[state_after - 1].0) / time_bracket;
            &self.states[state_after - 1].1 * interp_t
                + &self.states[state_after].1 * (1.0 - interp_t)
        } else {
            self.states[0].1.clone()
        };

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

    pub fn push_circle(&mut self, circle: (f32, f32), velocity: (f32, f32)) -> bool {
        if circle.0 <= -1.0 + self.radius || circle.0 >= 1.0 - self.radius {
            return false;
        }
        if circle.1 <= -1.0 + self.radius * self.aspect_ratio
            || circle.1 >= 1.0 - self.radius * self.aspect_ratio
        {
            return false;
        }

        if let Some(last_state) = self.states.back() {
            for center in last_state.1.as_slice().chunks_exact(2) {
                let difference = (
                    circle.0 - center[0],
                    (circle.1 - center[1]) / self.aspect_ratio,
                );
                if difference.0.powi(2) + difference.1.powi(2) <= 4.0 * self.radius.powi(2) {
                    return false;
                }
            }

            let mut last_state = last_state.1.clone();
            last_state.extend([circle.0, circle.1]);
            self.velocities
                .borrow_mut()
                .extend([velocity.0, velocity.1]);
            self.reset(last_state).unwrap();
        } else {
            self.reset(BVector::from_column_slice_generic(
                Dyn(2),
                U1::name(),
                &[circle.0, circle.1],
            ))
            .unwrap();
            *self.velocities.borrow_mut() =
                BVector::from_column_slice_generic(Dyn(2), U1::name(), &[velocity.0, velocity.1]);
        }

        true
    }

    pub fn pop(&mut self, num: usize) {
        let last_state = self.states.back().unwrap();
        let last_len = last_state.1.len();
        let last_state = last_state
            .1
            .clone()
            .remove_rows(last_len - num * 2, num * 2);

        let velocities = self.velocities.borrow().clone();
        let velocities = velocities.remove_rows(last_len - num * 2, num * 2);
        *self.velocities.borrow_mut() = velocities;
        self.reset(last_state).unwrap();
    }
}
