import { apiClient } from './client'
import type {
  AdminProxyNode,
  AdminProxyNodeMutationResponse,
  AdminProxyPoolResponse,
  AdminProxyPoolSettingsResponse,
  AdminProxyTestResponse,
  CreateAdminProxyNodeRequest,
  UpdateAdminProxyNodeRequest,
  UpdateAdminProxyPoolSettingsRequest,
} from './types'

export type ProxyNode = AdminProxyNode

export const proxiesApi = {
  listProxies: () => apiClient.get<AdminProxyPoolResponse>('/admin/proxies'),
  createProxy: (payload: CreateAdminProxyNodeRequest) =>
    apiClient.post<AdminProxyNodeMutationResponse>('/admin/proxies', payload),
  updateProxy: (proxyId: string, payload: UpdateAdminProxyNodeRequest) =>
    apiClient.put<AdminProxyNodeMutationResponse>(`/admin/proxies/${proxyId}`, payload),
  deleteProxy: (proxyId: string) => apiClient.delete<void>(`/admin/proxies/${proxyId}`),
  updateSettings: (payload: UpdateAdminProxyPoolSettingsRequest) =>
    apiClient.put<AdminProxyPoolSettingsResponse>('/admin/proxies/settings', payload),
  testAll: () => apiClient.post<AdminProxyTestResponse>('/admin/proxies/test'),
  testProxy: (proxyId: string) =>
    apiClient.post<AdminProxyTestResponse>('/admin/proxies/test', undefined, {
      params: { proxy_id: proxyId },
    }),
}
