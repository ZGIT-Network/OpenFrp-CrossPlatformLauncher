<script setup lang="ts">
import { NCard, NAlert, NSpace, NCollapse, NCollapseItem, NList, NListItem, NThing, NTable, NScrollbar, NImage, NGradientText } from 'naive-ui';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const currentVersion = ref('v0.1')

const getCurrentVersion = async () => {
    try {
        const version = await invoke('get_cpl_version')
        currentVersion.value = version as string
    } catch (e) {
        currentVersion.value = '获取失败'
        console.error('获取版本失败:', e)
    }
}
getCurrentVersion()

</script>
<template>
    <n-scrollbar>
        <n-space vertical>
            <n-alert title="技术测试版本警告"
                type="warning">当前版本属于非常早期的技术测试版本，可能存在很多问题，请谨慎在生产环境使用。<br />若遇到问题，请及时联系我们。</n-alert>
            <n-card title="关于 OpenFrp Cross Platform Launcher">
                <n-table striped>

                    <tbody>
                        <tr>
                            <td>OpenFrp Cross Platform Launcher</td>
                            <td>v{{ currentVersion }}</td>
                        </tr>
                    </tbody>
                </n-table>
                <br />
                感谢您使用OpenFrp Cross Platform Launcher。<br />
                本项目是 OpenFrp 的跨平台启动器，基于新技术 Tauri 2 开发。<br /><br />
                得益于 Tauri2 的强大能力，本项目可以跨平台使用，支持 Windows、MacOS、Linux 等操作系统。<br />
                由于相关问题过多，我们暂时不准备开源本项目，但欢迎开发者们联系我们参与到本项目的开发中。<br />
                <br />
                作者：云默安 <br />
                Copyright © 2025 ZGIT Network. All rights reserved.
            </n-card>
            <n-card>
                <n-collapse>
                    <n-collapse-item title="赞助开发" name="1">
                        <template #header-extra>
                            <n-gradient-text type="error">
                                快吃不起饭了求你了给点吧.jpg
                              </n-gradient-text>
                        </template>
                        <n-text>
                            感谢使用。<br />
                            如果您愿意赞助本项目，请扫描下方二维码，或者前往 <a href="https://afdian.com/a/zgitnetwork">爱发电</a> 赞助。<br />您的每一笔赞助都将用于本项目的开发并标注在 <a href="https://www.zyghit.cn/sponsor">赞助者列表</a>。<br />
                            <n-image width="300" src="https://r.zyghit.cn/images/ymawx.png" /><n-image width="200" src="https://r.zyghit.cn/images/ymaalyp.png" /><br />
                        </n-text>
                    </n-collapse-item>
                </n-collapse>
            </n-card>
            <n-card>
                <n-collapse>
                    <n-collapse-item title="使用的开源组件 (NPM)" name="1">
                        <n-list hoverable>
                            <n-list-item>
                                <n-thing title="Tauri-App@^2.2.0" description="https://github.com/tauri-apps/tauri" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="naive-ui@^2.41.0" description="https://github.com/tusen-ai/naive-ui" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="axios@^1.7.9" description="https://github.com/axios/axios" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="less@^4.2.2" description="https://github.com/less/less.js" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="marked@^15.0.6" description="https://github.com/markedjs/marked" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="uuid@^11.0.5" description="https://github.com/uuidjs/uuid" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="vue@^3.5.13" description="https://github.com/vuejs/vue" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="vue-router@^4.5.0" description="https://github.com/vuejs/router" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="vfonts@^0.0.3" description="https://github.com/07akioni/vfonts" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="typescript@~5.6.2"
                                    description="https://github.com/microsoft/TypeScript" />
                            </n-list-item>
                            <n-list-item>
                                <n-thing title="vite@^6.0.3" description="https://github.com/vitejs/vite" />
                            </n-list-item>
                        </n-list>
                    </n-collapse-item>
                </n-collapse>
            </n-card>
        </n-space>
    </n-scrollbar>
</template>