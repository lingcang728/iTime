<script lang="ts">
import type { InjectionKey } from 'vue'

export interface BillingCheckoutRequest {
  plan: 'pro'
  source: 'pro-page'
}

export type BillingCheckoutHook = (request: BillingCheckoutRequest) => Promise<void>

export const BILLING_CHECKOUT_HOOK: InjectionKey<BillingCheckoutHook> = Symbol('itime-billing-checkout')
</script>

<script setup lang="ts">
import { computed, inject } from 'vue'
import { PhArrowLeft, PhCheck, PhLockKey, PhSparkle } from '@phosphor-icons/vue'
import { useRouter } from 'vue-router'

const router = useRouter()
const billingCheckout = inject(BILLING_CHECKOUT_HOOK, null)
const billingReady = computed(() => billingCheckout !== null)

async function beginCheckout(): Promise<void> {
  if (!billingCheckout) return
  await billingCheckout({ plan: 'pro', source: 'pro-page' })
}
</script>

<template>
  <section class="page account-page pro-page">
    <header class="account-header">
      <button class="account-back" type="button" aria-label="返回账户页" @click="router.push({ name: 'account' })"><PhArrowLeft :size="18" /></button>
      <div><span>iTime Pro</span><h1>敬请期待</h1><p>当前账户继续保持 Free，不会自动升级或扣费。</p></div>
    </header>

    <article class="pro-hero">
      <div class="pro-mark"><PhSparkle :size="30" weight="duotone" /></div>
      <span>未来能力预留</span>
      <h2>更深入的长期回顾，仍在认真打磨</h2>
      <p>Pro 入口已经预留标准结算钩子，但在正式接入服务前保持关闭。</p>
      <button class="button primary pro-cta" type="button" :disabled="!billingReady" @click="beginCheckout">
        {{ billingReady ? '继续升级' : '敬请期待' }}
      </button>
      <small><PhLockKey :size="14" />当前没有连接任何支付服务</small>
    </article>

    <div class="pro-preview-grid" aria-label="计划中的 Pro 能力">
      <article><PhCheck :size="17" /><div><strong>更长周期回顾</strong><span>跨月查看时间结构的变化。</span></div></article>
      <article><PhCheck :size="17" /><div><strong>自定义报告</strong><span>按自己的关注点组织周报。</span></div></article>
      <article><PhCheck :size="17" /><div><strong>本地数据导出</strong><span>以清晰格式带走自己的记录。</span></div></article>
    </div>
  </section>
</template>

<style scoped src="../styles/account-pages.css"></style>
<style scoped src="../styles/pro-page.css"></style>
