export function formatDuration(value: number | null, compact = false): string {
  if (value === null) return '暂无数据'
  const totalMinutes = Math.round(value / 60_000)
  if (value > 0 && totalMinutes === 0) return compact ? '< 1 分钟' : '不足1分钟'
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  if (!hours) return compact ? `${minutes} 分钟` : `${minutes}分钟`
  if (!minutes) return compact ? `${hours} 小时` : `${hours}小时`
  return compact ? `${hours} 小时 ${minutes} 分` : `${hours}小时${minutes}分钟`
}

export function formatRatio(value: number | null): string {
  return value === null ? '暂无数据' : `${value.toFixed(1)}×`
}

export function formatNumber(value: number | null): string {
  return value === null ? '暂无数据' : Math.round(value).toLocaleString('zh-CN')
}

export function formatDistance(pixels: number): string {
  const meters = pixels / 3779.527559
  return meters >= 1000 ? `${(meters / 1000).toFixed(1)} km` : `${meters.toFixed(1)} m`
}

export function formatDateLabel(date: string): string {
  const value = new Date(`${date}T12:00:00`)
  const week = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'][value.getDay()]
  return `${value.getMonth() + 1}月${value.getDate()}日 ${week}`
}

export function formatClock(value: number): string {
  const date = new Date(value)
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`
}
