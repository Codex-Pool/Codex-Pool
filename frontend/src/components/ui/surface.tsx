import type { ComponentProps, HTMLAttributes, ReactNode } from 'react'

import { Divider } from '@heroui/react'

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { resolvePanelShellClassName } from '@/lib/panel-shell'
import { cn } from '@/lib/utils'

type SurfaceTone = 'default' | 'muted' | 'brand' | 'success' | 'warning' | 'danger'

const SURFACE_TONE_CLASS_NAMES: Record<SurfaceTone, string> = {
  default: `${resolvePanelShellClassName('primary')} text-foreground`,
  muted: `${resolvePanelShellClassName('secondary')} text-foreground`,
  brand: 'border-small border-primary/20 bg-primary/[0.05] text-foreground dark:bg-primary/[0.08]',
  success: 'border-small border-success/20 bg-success/[0.05] text-foreground dark:bg-success/[0.08]',
  warning: 'border-small border-warning/20 bg-warning/[0.05] text-foreground dark:bg-warning/[0.08]',
  danger: 'border-small border-danger/20 bg-danger/[0.05] text-foreground dark:bg-danger/[0.08]',
}

interface SurfaceCardProps extends Omit<ComponentProps<typeof Card>, 'tone'> {
  tone?: SurfaceTone
}

export function SurfaceCard({
  tone = 'default',
  shadow,
  className,
  ...props
}: SurfaceCardProps) {
  return (
    <Card
      shadow={shadow ?? 'none'}
      className={cn(SURFACE_TONE_CLASS_NAMES[tone], className)}
      {...props}
    />
  )
}

export function SurfaceCardBody({
  className,
  ...props
}: ComponentProps<typeof CardContent>) {
  return <CardContent className={cn('space-y-4 p-4 sm:p-5', className)} {...props} />
}

export function SurfaceCardHeader({
  className,
  ...props
}: ComponentProps<typeof CardHeader>) {
  return <CardHeader className={cn('px-4 py-4 sm:px-5 sm:py-4', className)} {...props} />
}

export function SurfaceSection({
  title,
  description,
  tone = 'muted',
  className,
  children,
}: {
  title?: ReactNode
  description?: ReactNode
  tone?: SurfaceTone
  className?: string
  children?: ReactNode
}) {
  return (
    <SurfaceCard tone={tone} shadow="none" className={className}>
      {title || description ? (
        <>
          <SurfaceCardHeader className="flex-col items-start gap-1.5">
            {title ? <CardTitle className="text-sm font-semibold">{title}</CardTitle> : null}
            {description ? (
              <div className="text-xs leading-6 text-default-500">{description}</div>
            ) : null}
          </SurfaceCardHeader>
          <SurfaceDivider />
        </>
      ) : null}
      <SurfaceCardBody>{children}</SurfaceCardBody>
    </SurfaceCard>
  )
}

export function SurfaceNotice({
  tone,
  className,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement> & {
  tone: Exclude<SurfaceTone, 'default' | 'muted'>
}) {
  return (
    <SurfaceCard tone={tone} shadow="none" className={className}>
      <SurfaceCardBody className="px-4 py-3 text-sm" {...props}>
        {children}
      </SurfaceCardBody>
    </SurfaceCard>
  )
}

export function SurfaceInset({
  tone = 'muted',
  className,
  children,
  ...props
}: {
  tone?: SurfaceTone
  className?: string
  children?: ReactNode
} & HTMLAttributes<HTMLDivElement>) {
  return (
    <SurfaceCard tone={tone} shadow="none">
      <SurfaceCardBody className="p-3">
        <div className={className} {...props}>
          {children}
        </div>
      </SurfaceCardBody>
    </SurfaceCard>
  )
}

export function SurfaceCode({
  className,
  children,
}: {
  className?: string
  children?: ReactNode
}) {
  return (
    <SurfaceInset tone="muted" className={className}>
      <pre className="overflow-auto whitespace-pre-wrap break-words font-mono text-xs leading-relaxed text-foreground">
        {children}
      </pre>
    </SurfaceInset>
  )
}

export function SurfaceDivider({
  className,
  ...props
}: ComponentProps<typeof Divider>) {
  return <Divider className={cn('bg-divider', className)} {...props} />
}
