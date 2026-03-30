import type { ComponentProps, HTMLAttributes, ReactNode } from 'react'

import { Divider } from '@heroui/react'

import { PagePanel } from '@/components/layout/page-archetypes'
import { useUiPreferences } from '@/components/use-ui-preferences'
import {
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import type { DrawerPlacement } from '@/lib/ui-preferences'
import { cn } from '@/lib/utils'

export type AntigravityDialogSize = 'sm' | 'lg' | 'xl'

const DIALOG_SIZE_CLASS_NAMES: Record<DrawerPlacement, Record<AntigravityDialogSize, string>> = {
  bottom: {
    sm: 'w-full rounded-t-large max-h-[min(82dvh,720px)]',
    lg: 'w-full rounded-t-large max-h-[min(86dvh,900px)]',
    xl: 'w-full rounded-t-large max-h-[96dvh]',
  },
  top: {
    sm: 'w-full rounded-b-large max-h-[min(82dvh,720px)]',
    lg: 'w-full rounded-b-large max-h-[min(86dvh,900px)]',
    xl: 'w-full rounded-b-large max-h-[96dvh]',
  },
  right: {
    sm: 'h-full w-[min(100vw,34rem)] max-w-[34rem]',
    lg: 'h-full w-[min(100vw,56rem)] max-w-[56rem]',
    xl: 'h-full w-[min(100vw,88rem)] max-w-[88rem]',
  },
  left: {
    sm: 'h-full w-[min(100vw,34rem)] max-w-[34rem]',
    lg: 'h-full w-[min(100vw,56rem)] max-w-[56rem]',
    xl: 'h-full w-[min(100vw,88rem)] max-w-[88rem]',
  },
}

type AntigravityDialogShellProps = Omit<
  ComponentProps<typeof DialogContent>,
  'children' | 'title'
> & {
  title: ReactNode
  description?: ReactNode
  meta?: ReactNode
  footer?: ReactNode
  size?: AntigravityDialogSize
  children?: ReactNode
  bodyClassName?: string
}

export function AntigravityDialogShell({
  title,
  description,
  meta,
  footer,
  size = 'lg',
  className,
  bodyClassName,
  children,
  showCloseButton = true,
  ...props
}: AntigravityDialogShellProps) {
  const { drawerPlacement } = useUiPreferences()

  return (
    <DialogContent
      className={cn(
        DIALOG_SIZE_CLASS_NAMES[drawerPlacement][size],
        'overflow-hidden border-small border-default-200 bg-content1 p-0',
        className,
      )}
      showCloseButton={showCloseButton}
      {...props}
    >
      <div
        className={cn(
          'flex flex-col',
          drawerPlacement === 'left' || drawerPlacement === 'right'
            ? 'h-full min-h-0'
            : 'max-h-[min(96dvh,calc(100dvh-0.25rem))]',
        )}
      >
        <DialogHeader className="shrink-0 bg-content2 px-4 py-4 pr-14 text-left sm:px-6 sm:py-5">
          <DialogTitle>{title}</DialogTitle>
          {description ? <DialogDescription>{description}</DialogDescription> : null}
          {meta ? <AntigravityDialogMeta className="pt-3">{meta}</AntigravityDialogMeta> : null}
        </DialogHeader>
        <Divider className="bg-divider" />

        <div
          className={cn(
            'min-h-0 flex-1 overflow-y-auto bg-content1 px-4 py-4 sm:px-6 sm:py-5',
            bodyClassName,
          )}
        >
          {children}
        </div>

        {footer ? (
          <>
            <Divider className="bg-divider" />
            <div className="shrink-0 bg-content1 px-4 py-4 sm:px-6">{footer}</div>
          </>
        ) : null}
      </div>
    </DialogContent>
  )
}

export function AntigravityDialogMeta({
  className,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        'flex flex-wrap items-center gap-2 text-xs text-muted-foreground',
        className,
      )}
      {...props}
    >
      {children}
    </div>
  )
}

export function AntigravityDialogBody({
  className,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn('space-y-4 sm:space-y-5', className)} {...props}>
      {children}
    </div>
  )
}

type AntigravityDialogPanelProps = ComponentProps<typeof PagePanel>

export function AntigravityDialogPanel({
  tone = 'primary',
  className,
  children,
  ...props
}: AntigravityDialogPanelProps) {
  return (
    <PagePanel
      tone={tone}
      className={cn('space-y-4 rounded-large p-4 sm:p-5', className)}
      {...props}
    >
      {children}
    </PagePanel>
  )
}

export function AntigravityDialogActions({
  className,
  children,
  ...props
}: ComponentProps<typeof DialogFooter>) {
  return (
    <DialogFooter className={cn('sm:items-center', className)} {...props}>
      {children}
    </DialogFooter>
  )
}
