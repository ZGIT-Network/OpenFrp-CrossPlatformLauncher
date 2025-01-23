<script lang="ts" setup>

import './style.less';


</script>

<template>
  <div class="header">
    <div class="header-left">
      <div class="header-logo">
        <div style="margin-top: 6px">
          <svg fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 361.29 51.55">
            <path
              d="M99.65,156.91A29,29,0,0,1,89,155a25.58,25.58,0,0,1-8.53-5.44,25,25,0,0,1-5.61-8.17,26.89,26.89,0,0,1,0-20.45,25,25,0,0,1,5.61-8.17,25.65,25.65,0,0,1,8.5-5.44,30.65,30.65,0,0,1,21.38,0,25.18,25.18,0,0,1,8.5,5.4,24.47,24.47,0,0,1,5.58,8.2,27.68,27.68,0,0,1,0,20.56,24.65,24.65,0,0,1-5.58,8.17,25.18,25.18,0,0,1-8.5,5.4A29.13,29.13,0,0,1,99.65,156.91Zm0-6.41a20.41,20.41,0,0,0,7.73-1.44,18.36,18.36,0,0,0,6.2-4.07,18.89,18.89,0,0,0,4.1-6.15,19.84,19.84,0,0,0,1.48-7.71,19.3,19.3,0,0,0-5.58-13.82,18.14,18.14,0,0,0-6.2-4.1,20.41,20.41,0,0,0-7.73-1.44,20.69,20.69,0,0,0-7.82,1.44,18.56,18.56,0,0,0-6.23,4.1A19.05,19.05,0,0,0,80,131.13a19.83,19.83,0,0,0,1.47,7.71,18.71,18.71,0,0,0,10.37,10.22A20.69,20.69,0,0,0,99.65,150.5Z"
              transform="translate(-72.79 -105.36)" />
            <path
              d="M137.37,156.33v-50.4H157a27.67,27.67,0,0,1,11.3,2.09,16,16,0,0,1,7.2,6,18.85,18.85,0,0,1,0,18.83,16.38,16.38,0,0,1-7.2,6,27.33,27.33,0,0,1-11.3,2.12h-15.7l3.24-3.38v18.64Zm7.2-17.92-3.24-3.6h15.48q6.91,0,10.48-3a11.57,11.57,0,0,0,0-16.7q-3.57-3-10.48-3H141.33l3.24-3.6Z"
              transform="translate(-72.79 -105.36)" />
            <path d="M195.84,150.07h29.37v6.26H188.64v-50.4H224.2v6.27H195.84Zm-.65-22.39h25.92v6.12H195.19Z"
              transform="translate(-72.79 -105.36)" />
            <path d="M236.88,156.33v-50.4h5.9l33.34,41.4H273v-41.4h7.2v50.4h-5.9L241,114.93h3.1v41.4Z"
              transform="translate(-72.79 -105.36)" />
            <path d="M302.54,156.33h-7.2v-50.4h35.57v6.27H302.54Zm-.72-26.56h26V136h-26Z"
              transform="translate(-72.79 -105.36)" />
            <path
              d="M341.06,156.33v-50.4h19.65A27.69,27.69,0,0,1,372,108a16,16,0,0,1,7.2,6,18.85,18.85,0,0,1,0,18.83,16.11,16.11,0,0,1-7.2,6A27.69,27.69,0,0,1,360.71,141H345l3.24-3.31v18.64Zm7.2-17.92L345,134.88H360.5q6.92,0,10.47-3a10.34,10.34,0,0,0,3.57-8.39,10.23,10.23,0,0,0-3.57-8.35q-3.56-3-10.47-3H345l3.24-3.6Zm26.49,17.92-12.81-18.28h7.7l13,18.28Z"
              transform="translate(-72.79 -105.36)" />
            <path
              d="M393.4,156.33v-50.4h19.66a27.67,27.67,0,0,1,11.3,2.09,16,16,0,0,1,7.2,6,18.85,18.85,0,0,1,0,18.83,16.38,16.38,0,0,1-7.2,6,27.33,27.33,0,0,1-11.3,2.12h-15.7l3.24-3.38v18.64Zm7.2-17.92-3.24-3.6h15.48q6.92,0,10.48-3a11.57,11.57,0,0,0,0-16.7q-3.57-3-10.48-3H397.36l3.24-3.6Z"
              transform="translate(-72.79 -105.36)" />
          </svg>
        </div>
        Cross Platform Launcher (Tauri)
      </div>
    </div>
    <div class="header-right">
      <n-text>
         v0.1 (Tech_Test)
      </n-text>
    </div>
  </div>

  
  
</template>
<style scoped>
.header {
  height: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 5px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.header-logo span {
  font-size: 18px;
  font-weight: bold;
  line-height: 28px;
}

.header-logo svg {
  width: 100px;
  height: auto;
}

.header-logo svg path .actual-light {
  color: rgb(0, 0, 0);
}

.header-logo svg path .actual-dark {
  color: rgb(255, 255, 255);
}

@media screen and (max-width: 280px) {
  .header-logo {
    display: none;
  }
}

.header-right {
  float: right;
  padding-right: 10px;
}


.header-userpanel {
  display: inline-block;
  padding: 16px 16px 16px 0;
}

.header-userpanel .header-userpanel-icon {
  vertical-align: middle;
}

.header-userpanel .header-userpanel-username {
  margin-left: 3px;
  font-size: 12px;
  vertical-align: middle;
}

.header-logout {
  display: inline-block;
  padding: 16px 16px 16px 0;
}

.header-logout .n-button {
  padding-left: 1px;
}

.header-avatar {
  padding: 15px 18px 10px 0;
}

</style>