import type { AdminSystemStateResponse } from '../../api/types.ts'

export type SystemComponentStatus = 'healthy' | 'degraded' | 'checking'

export interface SystemComponentRow {
  id: 'control-plane' | 'data-plane' | 'usage-repo'
  name: string
  status: SystemComponentStatus
  description: string
}

type SystemStateLike = Pick<
  AdminSystemStateResponse,
  'usage_repo_available' | 'data_plane_error' | 'data_plane_debug'
>

export function resolveSystemComponentRows(
  state: SystemStateLike | undefined,
  t: (key: string) => string,
): SystemComponentRow[] {
  return [
    {
      id: 'control-plane',
      name: t('system.antigravity.components.controlPlane.name'),
      status: 'healthy',
      description: t('system.antigravity.components.controlPlane.description'),
    },
    {
      id: 'data-plane',
      name: t('system.antigravity.components.dataPlane.name'),
      status: state?.data_plane_error
        ? 'degraded'
        : state?.data_plane_debug
          ? 'healthy'
          : 'checking',
      description: state?.data_plane_error
        ? state.data_plane_error
        : state?.data_plane_debug
          ? t('system.antigravity.components.dataPlane.connected')
          : t('system.antigravity.components.dataPlane.waiting'),
    },
    {
      id: 'usage-repo',
      name: t('system.antigravity.components.usageRepo.name'),
      status: state?.usage_repo_available ? 'healthy' : 'degraded',
      description: state?.usage_repo_available
        ? t('system.antigravity.components.usageRepo.available')
        : t('system.antigravity.components.usageRepo.unavailable'),
    },
  ]
}

export function formatDurationFromSeconds(
  totalSeconds: number | undefined,
  unknownLabel: string,
): string {
  if (typeof totalSeconds !== 'number' || !Number.isFinite(totalSeconds) || totalSeconds < 0) {
    return unknownLabel
  }

  const days = Math.floor(totalSeconds / 86400)
  const hours = Math.floor((totalSeconds % 86400) / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)

  if (days > 0) {
    return `${days}d ${hours}h`
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }
  return `${minutes}m`
}
