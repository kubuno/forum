// Bridge so non-component callbacks (e.g. the host search bar's onSearch) can use
// the module's react-router navigation. The sidebar — mounted for every /forum
// route — registers the live navigate function here.
let navFn: ((to: string) => void) | null = null

export function setNav(fn: (to: string) => void) {
  navFn = fn
}

export function goTo(to: string) {
  navFn?.(to)
}
