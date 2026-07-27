(() => {
  const releaseApi = 'https://api.github.com/repos/lingcang728/iTime/releases/latest'
  const releasesPage = 'https://github.com/lingcang728/iTime/releases/latest'
  const toggle = document.querySelector('.nav-toggle')
  const mobileNav = document.getElementById('mobile-nav')

  if (toggle && mobileNav) {
    const setOpen = (open) => {
      toggle.setAttribute('aria-expanded', String(open))
      toggle.setAttribute('aria-label', open ? '关闭菜单' : '打开菜单')
      mobileNav.hidden = !open
    }

    toggle.addEventListener('click', () => {
      setOpen(toggle.getAttribute('aria-expanded') !== 'true')
    })
    mobileNav.querySelectorAll('a').forEach((link) => {
      link.addEventListener('click', () => setOpen(false))
    })
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && toggle.getAttribute('aria-expanded') === 'true') {
        setOpen(false)
        toggle.focus()
      }
    })
    document.addEventListener('pointerdown', (event) => {
      if (
        toggle.getAttribute('aria-expanded') === 'true'
        && event.target instanceof Node
        && !toggle.contains(event.target)
        && !mobileNav.contains(event.target)
      ) {
        setOpen(false)
      }
    })
  }

  function setText(selector, value) {
    document.querySelectorAll(selector).forEach((element) => {
      element.textContent = value
    })
  }

  function formatBytes(bytes) {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`
  }

  function trustedGithubUrl(value, prefix) {
    const url = new URL(value)
    if (url.protocol !== 'https:' || url.hostname !== 'github.com' || !url.pathname.startsWith(prefix)) {
      throw new Error('Release 返回了非预期下载地址')
    }
    return url.href
  }

  function releaseAsset(release, role) {
    const version = release.tag_name.slice(1)
    const match = role === 'portable'
      ? (asset) => asset.name === 'iTime.exe'
      : (asset) => asset.name === `iTime_${version}_x64-setup.exe`
    const asset = release.assets.find(match)
    if (!asset || !Number.isSafeInteger(asset.size) || asset.size <= 0) {
      throw new Error(`最新 Release 缺少${role === 'portable' ? '便携版' : '安装版'}资产`)
    }
    const sha256 = typeof asset.digest === 'string' && /^sha256:[0-9a-f]{64}$/i.test(asset.digest)
      ? asset.digest.slice(7).toLowerCase()
      : null
    if (!sha256) throw new Error(`${asset.name} 缺少 GitHub SHA-256 digest`)
    return {
      name: asset.name,
      size: asset.size,
      sha256,
      downloadUrl: trustedGithubUrl(
        asset.browser_download_url,
        `/lingcang728/iTime/releases/download/${release.tag_name}/`,
      ),
    }
  }

  function applyAsset(role, asset) {
    setText(`[data-release-file="${role}"]`, asset.name)
    setText(`[data-release-size="${role}"]`, formatBytes(asset.size))
    setText(`[data-release-sha="${role}"]`, asset.sha256)
    document.querySelectorAll(`[data-release-link="${role}"]`).forEach((link) => {
      link.href = asset.downloadUrl
    })
    document.querySelectorAll(`[data-copy-role="${role}"]`).forEach((button) => {
      button.dataset.copy = asset.sha256
      button.disabled = false
    })
  }

  async function loadLatestRelease() {
    const status = document.getElementById('release-status')
    try {
      const response = await fetch(releaseApi, {
        headers: { Accept: 'application/vnd.github+json' },
      })
      if (!response.ok) throw new Error(`GitHub Release API 返回 ${response.status}`)
      const release = await response.json()
      if (!/^v\d+\.\d+\.\d+$/.test(release.tag_name) || !Array.isArray(release.assets)) {
        throw new Error('GitHub Release 元数据格式不符合预期')
      }
      const installer = releaseAsset(release, 'installer')
      const portable = releaseAsset(release, 'portable')
      const releasePage = trustedGithubUrl(
        release.html_url,
        `/lingcang728/iTime/releases/tag/${release.tag_name}`,
      )
      const published = new Date(release.published_at)
      if (Number.isNaN(published.getTime())) throw new Error('Release 发布时间无效')

      setText('[data-release-version]', release.tag_name)
      setText('[data-release-date]', new Intl.DateTimeFormat('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      }).format(published))
      document.querySelectorAll('[data-release-page]').forEach((link) => {
        link.href = releasePage
      })
      applyAsset('installer', installer)
      applyAsset('portable', portable)
      setText(
        '[data-release-command]',
        `Get-FileHash .\\${installer.name} -Algorithm SHA256\nGet-FileHash .\\${portable.name} -Algorithm SHA256`,
      )
      if (status) status.textContent = `已从 ${release.tag_name} 读取两个资产的文件名、大小和 SHA-256。`
    } catch (error) {
      document.querySelectorAll('[data-release-link]').forEach((link) => {
        link.href = releasesPage
      })
      if (status) {
        status.textContent = `暂时无法验证最新发布元数据：${error instanceof Error ? error.message : String(error)}。请在 GitHub Releases 页面下载并核对 digest。`
      }
    }
  }

  document.querySelectorAll('[data-copy-role]').forEach((button) => {
    button.addEventListener('click', async () => {
      const value = button.dataset.copy || ''
      if (!/^[0-9a-f]{64}$/i.test(value)) return
      const label = button.textContent
      try {
        await navigator.clipboard.writeText(value)
        button.textContent = '已复制'
        button.classList.add('is-copied')
        window.setTimeout(() => {
          button.textContent = label
          button.classList.remove('is-copied')
        }, 1600)
      } catch {
        const range = document.createRange()
        const code = button.closest('.hash-row')?.querySelector('.hash-value')
        if (code) {
          range.selectNodeContents(code)
          const selection = window.getSelection()
          selection?.removeAllRanges()
          selection?.addRange(range)
        }
        button.textContent = '请手动复制'
        window.setTimeout(() => {
          button.textContent = label
        }, 1800)
      }
    })
  })

  void loadLatestRelease()
})()
