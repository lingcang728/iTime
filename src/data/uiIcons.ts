/** Custom 3D UI icons extracted from the iTime icon system sheets. */
import brandItime from '../assets/ui-icons/brand-itime.png'
import pageHome from '../assets/ui-icons/page-home.png'
import pageAi from '../assets/ui-icons/page-ai.png'
import pageTimeline from '../assets/ui-icons/page-timeline.png'
import pageInput from '../assets/ui-icons/page-input.png'
import pageWeekly from '../assets/ui-icons/page-weekly.png'
import pageGoals from '../assets/ui-icons/page-goals.png'
import pageSettings from '../assets/ui-icons/page-settings.png'
import sectionRanking from '../assets/ui-icons/section-ranking.png'
import metricComputer from '../assets/ui-icons/metric-computer.png'
import metricAttention from '../assets/ui-icons/metric-attention.png'
import metricAiAgent from '../assets/ui-icons/metric-ai-agent.png'
import metricVoice from '../assets/ui-icons/metric-voice.png'
import metricMedia from '../assets/ui-icons/metric-media.png'
import metricCoverage from '../assets/ui-icons/metric-coverage.png'
import metricLeverage from '../assets/ui-icons/metric-leverage.png'
import metricConcurrency from '../assets/ui-icons/metric-concurrency.png'
import metricParallel from '../assets/ui-icons/metric-parallel.png'
import inputKeystrokes from '../assets/ui-icons/input-keystrokes.png'
import inputLeftClick from '../assets/ui-icons/input-left-click.png'
import inputRightClick from '../assets/ui-icons/input-right-click.png'
import inputMouseMove from '../assets/ui-icons/input-mouse-move.png'
import inputScroll from '../assets/ui-icons/input-scroll.png'
import inputHeatmap from '../assets/ui-icons/input-heatmap.png'
import inputTopKeys from '../assets/ui-icons/input-top-keys.png'
import inputShortcuts from '../assets/ui-icons/input-shortcuts.png'
import inputDataSource from '../assets/ui-icons/input-data-source.png'
import goalLearning from '../assets/ui-icons/goal-learning.png'
import goalDevelopment from '../assets/ui-icons/goal-development.png'
import goalAi from '../assets/ui-icons/goal-ai.png'
import goalContinuous from '../assets/ui-icons/goal-continuous.png'
import goalQuiet from '../assets/ui-icons/goal-quiet.png'
import goalAiNotify from '../assets/ui-icons/goal-ai-notify.png'
import weeklyAchievements from '../assets/ui-icons/weekly-achievements.png'
import weeklyBestDay from '../assets/ui-icons/weekly-best-day.png'
import goalSave from '../assets/ui-icons/goal-save.png'

export const uiIcons = {
  brandItime,
  pageHome,
  pageAi,
  pageTimeline,
  pageInput,
  pageWeekly,
  pageGoals,
  pageSettings,
  sectionRanking,
  metricComputer,
  metricAttention,
  metricAiAgent,
  metricVoice,
  metricMedia,
  metricCoverage,
  metricLeverage,
  metricConcurrency,
  metricParallel,
  inputKeystrokes,
  inputLeftClick,
  inputRightClick,
  inputMouseMove,
  inputScroll,
  inputHeatmap,
  inputTopKeys,
  inputShortcuts,
  inputDataSource,
  goalLearning,
  goalDevelopment,
  goalAi,
  goalContinuous,
  goalQuiet,
  goalAiNotify,
  weeklyAchievements,
  weeklyBestDay,
  goalSave,
  /** AI metric aliases */
  aiEffective: goalAi,
  aiCoverage: metricCoverage,
  aiLeverage: metricLeverage,
  aiConcurrency: metricConcurrency,
} as const

export type UiIconKey = keyof typeof uiIcons
