<script setup lang="ts">
import { NCard, NAlert, NSpace, NCollapse, NCollapseItem, NList, NListItem, NThing, NTable, NScrollbar, NImage, NGradientText, NText, NConfigProvider } from 'naive-ui';
import { ref, onMounted, inject } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { darkTheme } from 'naive-ui'

import cardImage from '@/assets/pexels-pok-rie-6135376.jpg';
import ymaalyp from '@/assets/ymaalyp.png';
import ymavx from '@/assets/ymawx.png';

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

const systemInfo = ref('')
const buildInfo = ref('')

onMounted(async () => {
    try {
        currentVersion.value = await invoke('get_cpl_version')
        systemInfo.value = await invoke('get_system_info')
        buildInfo.value = await invoke('get_build_info')
    } catch (e) {
        console.error('获取版本信息失败:', e)
    }
})
</script>
<template>
    <n-scrollbar>
        <n-space vertical>
            <!-- <n-alert type="warning">您当前正在使用 Beta 测试版本，可能存在一些问题，请谨慎在生产环境使用。<br />若遇到问题，请及时反馈。</n-alert> -->

            <n-config-provider :theme="darkTheme">
                <n-card :bordered="false" :style="{ background: `url(${cardImage}),rgba(0, 0, 0, 0.2)` }" style="
                  background-blend-mode: darken;
                  background-size: cover;
                  background-repeat: no-repeat;
                  border-radius: 4px;
                ">
                    <n-space :size="[0, 0]" :vertical="true" :style="{ 'margin-bottom': '18px' }">
                        <n-gradient-text :size="26" :gradient="{
                            deg: 135,
                            from: 'rgb(82 117 255)',
                            to: 'rgb(166 203 255) ',
                            }"> OpenFrp Ocean</n-gradient-text>
                        <n-text :depth="2" style="margin-bottom: 0;color:#fff">
                            欢迎使用 OpenFrp Cross Platform Launcher 跨平台启动器<br/>基于 Tauri 技术构建<br/>
                        </n-text>
                    </n-space>
                </n-card>
            </n-config-provider>
            
            <n-card title="关于 OpenFrp Cross Platform Launcher">
                <n-table striped>

                    <tbody>
                        <tr>
                            <td>OpenFrp Cross Platform Launcher</td>
                            <td>Beta v{{ currentVersion }}</td>
                        </tr>
                        <tr>
                            <td>系统架构</td>
                            <td>{{ systemInfo }}</td>
                        </tr>
                        <tr>
                            <td>构建号</td>
                            <td>{{ buildInfo }}</td>
                        </tr>
                    </tbody>
                </n-table>
                <br />
                感谢您使用 OpenFrp Cross Platform Launcher。<br />
                本项目是 OpenFrp 的跨平台启动器，基于新技术 Tauri 2 开发。<br /><br />
                
                得益于 Tauri2 的强大能力，本项目可以跨平台使用，支持 Windows、MacOS、Linux 等操作系统。<br /><br />

                <div v-external>
                    源代码地址：<a href="https://github.com/ZGIT-Network/OpenFrp-CrossPlatformLauncher">https://github.com/ZGIT-Network/OpenFrp-CrossPlatformLauncher</a><br />
                    本项目以 Apache-2.0 WITH Commons-Clause 协议开放源代码，有关附加条款请参考代码库。<br />
                    <br />
                    作者&软件著作权所有人：云默安 @至远光辉信息技术（天津）有限公司 <br />
                    特别鸣谢：Kent Ye<br/>
                    
                    Copyright © 2025 ZGIT Network. All rights reserved.
                </div>
            </n-card>
            <n-card>
                <n-collapse :default-expanded-names="['1']">
                    <n-collapse-item title="赞助开发" name="1">
                        <template #header-extra>
                            <n-gradient-text type="error">
                                快吃不起饭了求你了给点吧.jpg
                            </n-gradient-text>
                        </template>
                        <div v-external>
                            <n-text>
                                感谢使用。<br />
                                如果您愿意赞助本项目，请扫描下方二维码，或者前往 <a href="https://afdian.com/a/zgitnetwork">爱发电</a>
                                赞助。<br />您的每一笔赞助都将用于本项目的开发并标注在 <a href="https://www.zyghit.cn/sponsor">赞助者列表</a>。<br />
                                <n-image width="300" :src="ymavx" /><n-image width="200"
                                    :src="ymaalyp" /><br />
                            </n-text>
                        </div>
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
