import { memo, useState } from 'react'
import { Box, Copy, Folder, Github, RefreshCw, Shield, Trash2 } from 'lucide-react'
import { toast } from 'sonner'
import type { TFunction } from 'i18next'
import type { ManagedSkill, ToolOption } from './types'

type GithubInfo = {
  label: string
  href: string
}

type SkillCardProps = {
  skill: ManagedSkill
  installedTools: ToolOption[]
  loading: boolean
  getGithubInfo: (url: string | null | undefined) => GithubInfo | null
  getSkillSourceLabel: (skill: ManagedSkill) => string
  formatRelative: (ms: number | null | undefined) => string
  onUpdate: (skill: ManagedSkill) => void
  onDelete: (skillId: string) => void
  onToggleTool: (skill: ManagedSkill, toolId: string) => void
  onOpenDetail: (skill: ManagedSkill) => void
  t: TFunction
}

const MAX_VISIBLE_BADGES = 5

const SkillCard = ({
  skill,
  installedTools,
  loading,
  getGithubInfo,
  getSkillSourceLabel,
  formatRelative,
  onUpdate,
  onDelete,
  onToggleTool,
  onOpenDetail,
  t,
}: SkillCardProps) => {
  const typeKey = skill.source_type.toLowerCase()
  const iconNode = typeKey.includes('git') ? (
    <Github size={20} />
  ) : typeKey.includes('local') ? (
    <Folder size={20} />
  ) : (
    <Box size={20} />
  )
  const github = getGithubInfo(skill.source_ref)
  const copyValue = (github?.href ?? skill.source_ref ?? '').trim()

  const handleCopy = async () => {
    if (!copyValue) return
    try {
      await navigator.clipboard.writeText(copyValue)
      toast.success(t('copied'))
    } catch {
      toast.error(t('copyFailed'))
    }
  }

  // Split tools into synced and remaining for badge display
  const syncedTools: { tool: ToolOption; target: (typeof skill.targets)[0] }[] = []
  const unsyncedTools: ToolOption[] = []
  for (const tool of installedTools) {
    const target = skill.targets.find((tgt) => tgt.tool === tool.id)
    if (target) {
      syncedTools.push({ tool, target })
    } else {
      unsyncedTools.push(tool)
    }
  }

  const [expanded, setExpanded] = useState(false)
  const needsCollapse = syncedTools.length > MAX_VISIBLE_BADGES
  const visibleSynced = expanded ? syncedTools : syncedTools.slice(0, MAX_VISIBLE_BADGES)
  const remainingCount = syncedTools.length - MAX_VISIBLE_BADGES

  return (
    <div className="skill-card">
      <div className="skill-icon">{iconNode}</div>
      <div className="skill-main">
        <div className="skill-header-row">
          <button
            type="button"
            className="skill-name clickable"
            onClick={() => onOpenDetail(skill)}
          >
            {skill.name}
          </button>
          {skill.asset_type && skill.asset_type !== 'skill' ? (
            <span className={`asset-type-badge asset-type-${skill.asset_type}`}>
              {t(`assetType${skill.asset_type === 'mcp_server' ? 'McpServer' : skill.asset_type === 'plugin' ? 'Plugin' : 'Executable'}`)}
            </span>
          ) : null}
          {skill.security_status ? (
            <span
              className={`security-indicator security-${skill.security_status}`}
              title={t(`security${skill.security_status === 'trusted' ? 'Trusted' : skill.security_status === 'flagged' ? 'Flagged' : 'Unknown'}`)}
            >
              <Shield size={14} />
            </span>
          ) : null}
        </div>
        {skill.description ? (
          <div className="skill-desc">{skill.description}</div>
        ) : null}
        <div className="skill-meta-row">
          {github ? (
            <div className="skill-source">
              <button
                className="repo-pill copyable"
                type="button"
                title={t('copy')}
                aria-label={t('copy')}
                onClick={() => void handleCopy()}
                disabled={!copyValue}
              >
                {github.label}
                <span className="copy-icon" aria-hidden="true">
                  <Copy size={12} />
                </span>
              </button>
            </div>
          ) : (
            <div className="skill-source">
              <button
                className="repo-pill copyable"
                type="button"
                title={t('copy')}
                aria-label={t('copy')}
                onClick={() => void handleCopy()}
                disabled={!copyValue}
              >
                <span className="mono">{getSkillSourceLabel(skill)}</span>
                <span className="copy-icon" aria-hidden="true">
                  <Copy size={12} />
                </span>
              </button>
            </div>
          )}
          <div className="skill-source time">
            <span className="dot">•</span>
            {formatRelative(skill.updated_at)}
          </div>
        </div>
        <div className={`tool-matrix${!expanded && needsCollapse ? ' collapsed' : ''}`}>
          {visibleSynced.map(({ tool, target }) => (
            <button
              key={`${skill.id}-${tool.id}`}
              type="button"
              className="tool-pill active"
              title={`${tool.label} (${target.sync_mode ?? t('unknown')})`}
              onClick={() => void onToggleTool(skill, tool.id)}
            >
              <span className="status-badge" />
              {tool.label}
            </button>
          ))}
          {needsCollapse && !expanded ? (
            <button
              type="button"
              className="tool-pill more-badge"
              onClick={() => setExpanded(true)}
            >
              {t('moreTools', { count: remainingCount })}
            </button>
          ) : null}
          {expanded &&
            unsyncedTools.map((tool) => (
              <button
                key={`${skill.id}-${tool.id}`}
                type="button"
                className="tool-pill inactive"
                title={tool.label}
                onClick={() => void onToggleTool(skill, tool.id)}
              >
                {tool.label}
              </button>
            ))}
        </div>
      </div>
      <div className="skill-actions-col">
        <button
          className="card-btn primary-action"
          type="button"
          onClick={() => onUpdate(skill)}
          disabled={loading}
          aria-label={t('update')}
        >
          <RefreshCw size={16} />
        </button>
        <button
          className="card-btn danger-action"
          type="button"
          onClick={() => onDelete(skill.id)}
          disabled={loading}
          aria-label={t('remove')}
        >
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  )
}

export default memo(SkillCard)
