(() => {
  const toggle = document.querySelector('.nav-toggle')
  const mobileNav = document.getElementById('mobile-nav')

  if (toggle && mobileNav) {
    const setOpen = (open) => {
      toggle.setAttribute('aria-expanded', String(open))
      toggle.setAttribute('aria-label', open ? '关闭菜单' : '打开菜单')
      mobileNav.hidden = !open
    }

    toggle.addEventListener('click', () => {
      const open = toggle.getAttribute('aria-expanded') !== 'true'
      setOpen(open)
    })

    mobileNav.querySelectorAll('a').forEach((link) => {
      link.addEventListener('click', () => setOpen(false))
    })
  }

  document.querySelectorAll('[data-copy]').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const value = btn.getAttribute('data-copy') || ''
      const label = btn.textContent
      try {
        await navigator.clipboard.writeText(value)
        btn.textContent = '已复制'
        btn.classList.add('is-copied')
        window.setTimeout(() => {
          btn.textContent = label
          btn.classList.remove('is-copied')
        }, 1600)
      } catch {
        const range = document.createRange()
        const code = btn.closest('.hash-row')?.querySelector('.hash-value')
        if (code) {
          range.selectNodeContents(code)
          const selection = window.getSelection()
          selection?.removeAllRanges()
          selection?.addRange(range)
        }
        btn.textContent = '请手动复制'
        window.setTimeout(() => {
          btn.textContent = label
        }, 1800)
      }
    })
  })
})()
