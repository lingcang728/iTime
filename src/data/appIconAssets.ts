import antigravityIcon from '../assets/apps/antigravity.png'
import chatgptIcon from '../assets/apps/chatgpt.png'
import claudeIcon from '../assets/apps/claude.svg'
import explorerIcon from '../assets/apps/explorer.svg'
import grokIcon from '../assets/apps/grok.svg'
import typelessIcon from '../assets/apps/typeless.png'
import youtubeIcon from '../assets/apps/youtube.png'

/**
 * Stable brand icons for known apps / CLI coding agents.
 * Keys must match `canonicalAppKey` output so AI tool ids like
 * `grok-build` / `claude-code` resolve here even without a desktop .exe.
 */
export const embeddedAppIcons: Record<string, string> = {
  antigravity: antigravityIcon,
  chatgpt: chatgptIcon,
  claude: claudeIcon,
  codex: chatgptIcon,
  explorer: explorerIcon,
  grok: grokIcon,
  typeless: typelessIcon,
  youtube: youtubeIcon,
}
