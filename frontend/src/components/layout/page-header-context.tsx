import { useEffect, useState } from 'react'
import { useOutletContext } from 'react-router-dom'
import type { ReactNode } from 'react'

export interface AppPageHeader {
  title: ReactNode
  description?: ReactNode
  actions?: ReactNode
  dockedActions?: ReactNode
  actionsClassName?: string
  dockedActionsClassName?: string
  mode?: 'static-shell' | 'dock-on-scroll'
}

interface PageHeaderOutletContext {
  setPageHeader: (header: AppPageHeader | null) => void
  setPageHeaderBodyVisible: (visible: boolean) => void
}

export function usePageHeader(header: AppPageHeader | null) {
  const { setPageHeader } = useOutletContext<PageHeaderOutletContext>()

  useEffect(() => {
    setPageHeader(header)
    return () => setPageHeader(null)
  }, [header, setPageHeader])
}

export function usePageHeaderDocking() {
  const { setPageHeaderBodyVisible } = useOutletContext<PageHeaderOutletContext>()
  const [node, setNode] = useState<HTMLDivElement | null>(null)

  useEffect(() => {
    const root = node?.closest('[data-app-scroll-root]') as HTMLDivElement | null

    if (!node || !root || typeof window === 'undefined') {
      setPageHeaderBodyVisible(true)
      return
    }

    let frame = 0

    const updateVisibility = () => {
      frame = 0
      const anchorRect = node.getBoundingClientRect()
      const rootRect = root.getBoundingClientRect()
      const introStillVisible = anchorRect.bottom > rootRect.top + 24
      setPageHeaderBodyVisible(introStillVisible)
    }

    const scheduleVisibilityUpdate = () => {
      if (frame) return
      frame = window.requestAnimationFrame(updateVisibility)
    }

    scheduleVisibilityUpdate()
    root.addEventListener('scroll', scheduleVisibilityUpdate, { passive: true })
    window.addEventListener('resize', scheduleVisibilityUpdate)

    return () => {
      if (frame) {
        window.cancelAnimationFrame(frame)
      }
      root.removeEventListener('scroll', scheduleVisibilityUpdate)
      window.removeEventListener('resize', scheduleVisibilityUpdate)
      setPageHeaderBodyVisible(true)
    }
  }, [node, setPageHeaderBodyVisible])

  return setNode
}
