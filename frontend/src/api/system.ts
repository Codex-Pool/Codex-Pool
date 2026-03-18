import { apiClient } from './client'
import { DEFAULT_SYSTEM_CAPABILITIES } from './system.defaults'
import type { SystemCapabilitiesResponse } from './types'

export { DEFAULT_SYSTEM_CAPABILITIES }

export const systemApi = {
  async getCapabilities(): Promise<SystemCapabilitiesResponse> {
    try {
      return await apiClient.get<SystemCapabilitiesResponse>('/system/capabilities')
    } catch {
      return DEFAULT_SYSTEM_CAPABILITIES
    }
  },
}
