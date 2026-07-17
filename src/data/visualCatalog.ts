import type { Component } from 'vue'
import { PhBookOpenText, PhCode, PhFolders, PhRobot, PhSparkle, PhVideoCamera } from '@phosphor-icons/vue'

export interface CategoryVisual {
  icon: Component
  gradient: string
  iconTone: string
}

export const categoryVisuals: Record<string, CategoryVisual> = {
  '开发': { icon: PhCode, gradient: 'linear-gradient(90deg, #4f7fe8, #7499f3)', iconTone: '#4f7fe8' },
  '学习': { icon: PhBookOpenText, gradient: 'linear-gradient(90deg, #43ad78, #72c997)', iconTone: '#43ad78' },
  'AI 工具': { icon: PhRobot, gradient: 'linear-gradient(90deg, #6c5bd1, #9a85ec)', iconTone: '#7362d7' },
  '效率工具': { icon: PhSparkle, gradient: 'linear-gradient(90deg, #42aaa8, #73cfca)', iconTone: '#45aaa8' },
  '视频': { icon: PhVideoCamera, gradient: 'linear-gradient(90deg, #e06d4f, #f29a6c)', iconTone: '#df7253' },
  '系统工具': { icon: PhFolders, gradient: 'linear-gradient(90deg, #ce9131, #edbd61)', iconTone: '#d09535' },
}

export const fallbackCategoryVisual: CategoryVisual = {
  icon: PhFolders,
  gradient: 'linear-gradient(90deg, #929792, #b5b9b5)',
  iconTone: '#858a85',
}
