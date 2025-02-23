<script setup lang="ts">
import { ref } from 'vue'
import { NAlert, NGradientText, useMessage, NCard, NSpace } from 'naive-ui'
import axios from 'axios'
import { marked } from 'marked'

const message = useMessage()


const boardCast = ref('加载中...');
const headAlert = ref();

const fetchBoardCast = async () => {
    try {
        const res = await axios.get(`https://api.openfrp.net/commonQuery/get`, {
            params: {
                key: 'help_info',
            }
        })

        headAlert.value = (res.data.data as any).headalert;
        boardCast.value = (res.data.data as any).cpl_announce;
        // console.log(boardCast.value) 

    } catch (e) {
        message.error(`获取公告失败: ${e}`)
    }
}

fetchBoardCast();
</script>

<template>
    <div>
        <span class="hometitle">欢迎回来</span><br />
        欢迎使用全新 <n-gradient-text type="info">
            OpenFrp Cross Platform Launcher
        </n-gradient-text> !
        <div style="margin-top: 10px;">
            <n-space vertical>
              <n-alert type="warning">您当前正在使用 Alpha 测试版本，可能存在很多问题，请谨慎在生产环境使用。<br />若遇到问题，请及时与开发则反馈。</n-alert>
               
                <div v-if="headAlert?.status" style="margin-bottom: -8px;">
                    <n-alert :title="headAlert.title" :type="headAlert.type" closable style="margin-bottom: 8px;">
                        <div v-html="headAlert.content"></div>
                    </n-alert>
                </div>
                <n-card title="系统公告">
                
                    <div class="markdown" style="margin-top: -16px" v-html="marked.parse(boardCast)"></div>
                  </n-card>
            </n-space>
        </div>
    </div>


</template>

<style scoped>
.hometitle {
    font-size: 25px;
}
.markdown p {
    margin: 0;
  }
  
  .markdown hr {
    border-color: #ffffff42;
  }
  
  .markdown .actual-dark hr {
    border-color: #3c3c3c;
  }
  
  .markdown .right-bottom-bottom {
    display: none !important;
  }
  .markdown pre {
    margin-top: 10px;
    margin-bottom: 10px;
    border-radius: 5px;
    box-shadow: rgba(0, 0, 0, 0.55) 0px 2px 10px;
  }
  
  .markdown pre::before {
    content: '';
    display: block;
    background: url('https://cdn.futo.design/bop-web/assets/code-bar.svg') 10px 10px / 40px no-repeat rgb(40, 44, 52);
    height: 30px;
    width: 100%;
    margin-bottom: -7px;
    border-radius: 5px 5px 0 0;
  }
  
  .markdown pre code {
    overflow-x: auto;
    padding: 16px;
    color: #abb2bf;
    display: -webkit-box;
    font-family: Operator Mono, Consolas, Monaco, Menlo, monospace;
    font-size: 15px;
    -webkit-overflow-scrolling: touch;
    padding-top: 15px;
    background: #282c34;
    border-radius: 5px;
  }
  
</style>
