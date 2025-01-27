import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useLinkTunnelsStore = defineStore('linkTunnels', () => {
  const linkLaunchedTunnels = ref<Set<string>>(new Set())

  const addLinkTunnel = (tunnelId: string) => {
    linkLaunchedTunnels.value.add(tunnelId)
  }

  const removeLinkTunnel = (tunnelId: string) => {
    linkLaunchedTunnels.value.delete(tunnelId)
  }

  const clearLinkTunnels = () => {
    linkLaunchedTunnels.value.clear()
  }

  return {
    linkLaunchedTunnels,
    addLinkTunnel,
    removeLinkTunnel,
    clearLinkTunnels
  }
}) 