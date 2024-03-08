export async function wait(millis) {
  await new Promise(resolve => setTimeout(() => resolve(), millis))
}

export async function createWorker(memory, ptr) {
  let resolver = undefined
  const wait = new Promise(resolve => resolver = resolve)
  const worker = new Worker('worker.js', { type: 'module', name: 'aftgraphsWorker' })

  worker.onerror = event => console.error('web worker had an error', event)
  worker.onmessage = async () => {
    worker.onmessage = undefined
    resolver?.()
  }

  worker.postMessage([memory, ptr])
  await wait
  return worker
}
