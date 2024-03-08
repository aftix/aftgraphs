import init, { worker_entry_point } from './{{}}.js'

self.onmessage = async event => {
  self.onmessage = undefined

  await init(undefined, event.data[0])
  postMessage(0)
  worker_entry_point(Number(event.data[1]))
}
