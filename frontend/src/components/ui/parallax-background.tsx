export function ParallaxBackground() {
  return (
    <div className="pointer-events-none fixed inset-0 z-0 overflow-hidden bg-background transition-colors duration-700">
      <div className="absolute inset-0 bg-[linear-gradient(180deg,rgba(255,255,255,0.58)_0%,rgba(242,238,231,0.72)_100%)] dark:bg-[linear-gradient(180deg,rgba(20,24,30,0.86)_0%,rgba(16,20,27,0.94)_100%)]" />
      <div className="absolute inset-x-0 top-0 h-[32vh] bg-[radial-gradient(72%_58%_at_50%_0%,rgba(106,118,140,0.11),transparent_74%)] dark:bg-[radial-gradient(72%_58%_at_50%_0%,rgba(129,142,165,0.14),transparent_74%)]" />
      <div className="absolute inset-x-0 bottom-0 h-[24vh] bg-[linear-gradient(180deg,transparent,rgba(219,214,205,0.22))] dark:bg-[linear-gradient(180deg,transparent,rgba(40,47,58,0.26))]" />
    </div>
  )
}
