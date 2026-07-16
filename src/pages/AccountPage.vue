<script setup lang="ts">
import { PhArrowLeft, PhCaretRight, PhCheckCircle, PhDatabase, PhShieldCheck, PhUserCircle } from '@phosphor-icons/vue'
import { RouterLink, useRouter } from 'vue-router'

interface LocalAccountProfile {
  name: string
  plan: 'Free'
  storage: 'local'
}

const router = useRouter()
const account: Readonly<LocalAccountProfile> = {
  name: '林小北',
  plan: 'Free',
  storage: 'local',
}
</script>

<template>
  <section class="page account-page">
    <header class="account-header">
      <button class="account-back" type="button" aria-label="返回上一页" @click="router.back()"><PhArrowLeft :size="18" /></button>
      <div><span>账户</span><h1>个人空间</h1><p>管理当前方案与本机数据边界</p></div>
    </header>

    <article class="account-profile-card">
      <img src="/src/assets/avatar-lin.svg" alt="林小北的头像" />
      <div><span>本机账户</span><h2>{{ account.name }}</h2><p>iTime 的活动记录保存在这台电脑上。</p></div>
      <strong class="account-plan-badge">{{ account.plan }}</strong>
    </article>

    <div class="account-grid">
      <article class="card account-info-card">
        <div class="account-card-icon green"><PhShieldCheck :size="22" weight="duotone" /></div>
        <div><span>数据位置</span><h2>本机优先</h2><p>账户页不会上传活动内容，也不会把 Free 状态伪装成云端订阅。</p></div>
        <strong><PhCheckCircle :size="16" weight="fill" />已启用</strong>
      </article>
      <article class="card account-info-card">
        <div class="account-card-icon blue"><PhDatabase :size="22" weight="duotone" /></div>
        <div><span>当前空间</span><h2>个人使用</h2><p>本机记录、目标和偏好均服务于当前 Windows 用户。</p></div>
        <strong><PhUserCircle :size="16" weight="fill" />仅自己</strong>
      </article>
    </div>

    <RouterLink class="plan-entry-card" to="/pro" aria-label="查看 Pro 预告">
      <div><span>升级入口</span><h2>iTime Pro</h2><p>付费功能尚未开放，现阶段不会发起付款或收集支付信息。</p></div>
      <div class="plan-entry-action"><strong>敬请期待</strong><PhCaretRight :size="17" /></div>
    </RouterLink>
  </section>
</template>

<style scoped src="../styles/account-pages.css"></style>
