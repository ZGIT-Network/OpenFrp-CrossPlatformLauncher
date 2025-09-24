import { invoke } from '@tauri-apps/api/core'

export interface RequestLoginData {
  authorization_url: string
  request_uuid: string
}

export const argoGeneratePublicKey = () =>
  invoke<string>('argo_generate_public_key')

export const argoRequestLogin = () =>
  invoke<RequestLoginData>('argo_request_login')

export const argoPollLogin = (request_uuid: string) =>
  invoke<string>('argo_poll_login', { request_uuid })

export const argoWaitAuthorization = (request_uuid: string) =>
  invoke<string>('argo_wait_authorization', { request_uuid, requestUuid: request_uuid })

export const argoCancelWait = (request_uuid: string) =>
  invoke<void>('argo_cancel_wait', { request_uuid, requestUuid: request_uuid })


