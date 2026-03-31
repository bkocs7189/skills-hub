import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ArrowLeft } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { Update } from '@tauri-apps/plugin-updater'
import type { DeployProfileDto } from './types'

type UpdateStatus = 'idle' | 'checking' | 'up-to-date' | 'available' | 'downloading' | 'done' | 'error'

type DeployProfileRules = {
  skill?: string[]
  mcp_server?: string[]
  plugin?: string[]
  executable?: string[]
}

const ASSET_TOOL_OPTIONS: { assetType: keyof DeployProfileRules; labelKey: string; tools: string[] }[] = [
  { assetType: 'skill', labelKey: 'deployProfileSkillTargets', tools: ['claude_code', 'cursor', 'gemini_cli', 'windsurf', 'github_copilot', 'roo_code', 'cline', 'goose', 'amp', 'codex'] },
  { assetType: 'mcp_server', labelKey: 'deployProfileMcpTargets', tools: ['claude_code', 'cursor', 'windsurf', 'cline', 'roo_code'] },
  { assetType: 'plugin', labelKey: 'deployProfilePluginTargets', tools: ['claude_code'] },
]

function parseRules(rulesJson: string): DeployProfileRules {
  try {
    return JSON.parse(rulesJson) as DeployProfileRules
  } catch {
    return {}
  }
}

type SettingsPageProps = {
  isTauri: boolean
  language: string
  storagePath: string
  gitCacheCleanupDays: number
  gitCacheTtlSecs: number
  themePreference: 'system' | 'light' | 'dark'
  githubToken: string
  onPickStoragePath: () => void
  onToggleLanguage: () => void
  onThemeChange: (nextTheme: 'system' | 'light' | 'dark') => void
  onGitCacheCleanupDaysChange: (nextDays: number) => void
  onGitCacheTtlSecsChange: (nextSecs: number) => void
  onClearGitCacheNow: () => void
  onGithubTokenChange: (token: string) => void
  deployProfiles: DeployProfileDto[]
  onCreateProfile: (name: string, rules: string, isDefault: boolean) => void
  onUpdateProfile: (id: string, name: string, rules: string, isDefault: boolean) => void
  onDeleteProfile: (id: string) => void
  onBack: () => void
  t: TFunction
}

type DeployProfileCardProps = {
  profile: DeployProfileDto
  profileCount: number
  onUpdate: (id: string, name: string, rules: string, isDefault: boolean) => void
  onDelete: (id: string) => void
  t: TFunction
}

const DeployProfileCard = memo(({ profile, profileCount, onUpdate, onDelete, t }: DeployProfileCardProps) => {
  const [localName, setLocalName] = useState(profile.name)
  const rules = useMemo(() => parseRules(profile.rules), [profile.rules])

  const handleToggleTool = useCallback(
    (assetType: keyof DeployProfileRules, toolId: string, checked: boolean) => {
      const current = rules[assetType] ?? []
      const next = checked
        ? [...current, toolId]
        : current.filter((id) => id !== toolId)
      const updated = { ...rules, [assetType]: next }
      onUpdate(profile.id, profile.name, JSON.stringify(updated), profile.isDefault)
    },
    [rules, onUpdate, profile.id, profile.name, profile.isDefault],
  )

  return (
    <div className="deploy-profile-card">
      <div className="deploy-profile-header">
        <input
          className="deploy-profile-name-input"
          value={localName}
          onChange={(e) => setLocalName(e.target.value)}
          onBlur={() => {
            if (localName !== profile.name && localName.trim()) {
              onUpdate(profile.id, localName.trim(), profile.rules, profile.isDefault)
            }
          }}
        />
        {profile.isDefault && (
          <span className="deploy-profile-default-badge">{t('deployProfileDefault')}</span>
        )}
      </div>
      <div className="deploy-profile-rules">
        {ASSET_TOOL_OPTIONS.map(({ assetType, labelKey, tools }) => (
          <div key={assetType} className="deploy-profile-rule-group">
            <div className="deploy-profile-rule-label">{t(labelKey)}</div>
            <div className="deploy-profile-tool-grid">
              {tools.map((toolId) => {
                const isChecked = (rules[assetType] ?? []).includes(toolId)
                return (
                  <label key={toolId} className="deploy-profile-tool-checkbox">
                    <input
                      type="checkbox"
                      checked={isChecked}
                      onChange={(e) => handleToggleTool(assetType, toolId, e.target.checked)}
                    />
                    <span>{t(`tools.${toolId}`, { defaultValue: toolId })}</span>
                  </label>
                )
              })}
            </div>
          </div>
        ))}
      </div>
      <div className="deploy-profile-actions">
        {!profile.isDefault && (
          <button
            className="btn btn-secondary btn-sm"
            type="button"
            onClick={() => onUpdate(profile.id, profile.name, profile.rules, true)}
          >
            {t('deployProfileSetDefault')}
          </button>
        )}
        {profileCount > 1 && (
          <button
            className="btn btn-secondary btn-sm deploy-profile-delete-btn"
            type="button"
            onClick={() => onDelete(profile.id)}
          >
            {t('deployProfileDelete')}
          </button>
        )}
      </div>
    </div>
  )
})

const SettingsPage = ({
  isTauri,
  language,
  storagePath,
  gitCacheCleanupDays,
  gitCacheTtlSecs,
  themePreference,
  onPickStoragePath,
  onToggleLanguage,
  onThemeChange,
  onGitCacheCleanupDaysChange,
  onGitCacheTtlSecsChange,
  onClearGitCacheNow,
  githubToken,
  onGithubTokenChange,
  deployProfiles,
  onCreateProfile,
  onUpdateProfile,
  onDeleteProfile,
  onBack,
  t,
}: SettingsPageProps) => {
  const [localToken, setLocalToken] = useState(githubToken)
  useEffect(() => {
    setLocalToken(githubToken)
  }, [githubToken])

  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>('idle')
  const [updateVersion, setUpdateVersion] = useState<string | null>(null)
  const [updateError, setUpdateError] = useState<string | null>(null)
  const updateRef = useRef<Update | null>(null)

  const handleCheckUpdate = useCallback(async () => {
    if (!isTauri) return
    setUpdateStatus('checking')
    setUpdateError(null)
    try {
      const { check } = await import('@tauri-apps/plugin-updater')
      const update = await check()
      if (update) {
        updateRef.current = update
        setUpdateVersion(update.version)
        setUpdateStatus('available')
      } else {
        setUpdateStatus('up-to-date')
      }
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err))
      setUpdateStatus('error')
    }
  }, [isTauri])

  const handleInstallUpdate = useCallback(async () => {
    const update = updateRef.current
    if (!update) return
    setUpdateStatus('downloading')
    setUpdateError(null)
    try {
      await update.downloadAndInstall()
      setUpdateStatus('done')
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err))
      setUpdateStatus('error')
    }
  }, [])

  const [appVersion, setAppVersion] = useState<string | null>(null)
  const versionText = useMemo(() => {
    if (!isTauri) return t('notAvailable')
    if (!appVersion) return t('unknown')
    return `v${appVersion}`
  }, [appVersion, isTauri, t])

  const loadAppVersion = useCallback(async () => {
    if (!isTauri) {
      setAppVersion(null)
      return
    }
    try {
      const { getVersion } = await import('@tauri-apps/api/app')
      const v = await getVersion()
      setAppVersion(v)
    } catch {
      setAppVersion(null)
    }
  }, [isTauri])

  useEffect(() => {
    void loadAppVersion()
    return () => { updateRef.current = null }
  }, [loadAppVersion])

  return (
    <div className="settings-page">
      <div className="detail-header">
        <button className="detail-back-btn" type="button" onClick={onBack}>
          <ArrowLeft size={16} />
          {t('detail.back')}
        </button>
        <div className="detail-skill-name">{t('settings')}</div>
      </div>
      <div className="settings-page-body">
        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-language">
            {t('interfaceLanguage')}
          </label>
          <div className="settings-select-wrap">
            <select
              id="settings-language"
              className="settings-select"
              value={language}
              onChange={(event) => {
                if (event.target.value !== language) {
                  onToggleLanguage()
                }
              }}
            >
              <option value="en">{t('languageOptions.en')}</option>
              <option value="zh">{t('languageOptions.zh')}</option>
            </select>
            <svg
              className="settings-select-caret"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M6 9l6 6 6-6" />
            </svg>
          </div>
        </div>

        <div className="settings-field">
          <label className="settings-label" id="settings-theme-label">
            {t('themeMode')}
          </label>
          <div className="settings-theme-options" role="group" aria-labelledby="settings-theme-label">
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === 'system' ? 'active' : ''
              }`}
              aria-pressed={themePreference === 'system'}
              onClick={() => onThemeChange('system')}
            >
              {t('themeOptions.system')}
            </button>
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === 'light' ? 'active' : ''
              }`}
              aria-pressed={themePreference === 'light'}
              onClick={() => onThemeChange('light')}
            >
              {t('themeOptions.light')}
            </button>
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === 'dark' ? 'active' : ''
              }`}
              aria-pressed={themePreference === 'dark'}
              onClick={() => onThemeChange('dark')}
            >
              {t('themeOptions.dark')}
            </button>
          </div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-storage">
            {t('skillsStoragePath')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-storage"
              className="settings-input mono"
              value={storagePath}
              readOnly
            />
            <button
              className="btn btn-secondary settings-browse"
              type="button"
              onClick={onPickStoragePath}
            >
              {t('browse')}
            </button>
          </div>
          <div className="settings-helper">{t('skillsStorageHint')}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-git-cache-days">
            {t('gitCacheCleanupDays')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-git-cache-days"
              className="settings-input"
              type="number"
              min={0}
              max={3650}
              step={1}
              value={gitCacheCleanupDays}
              onChange={(event) => {
                const next = Number(event.target.value)
                if (!Number.isNaN(next)) {
                  onGitCacheCleanupDaysChange(next)
                }
              }}
            />
            <button
              className="btn btn-secondary settings-browse"
              type="button"
              onClick={onClearGitCacheNow}
            >
              {t('cleanNow')}
            </button>
          </div>
          <div className="settings-helper">{t('gitCacheCleanupHint')}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-git-cache-ttl">
            {t('gitCacheTtlSecs')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-git-cache-ttl"
              className="settings-input"
              type="number"
              min={0}
              max={3600}
              step={1}
              value={gitCacheTtlSecs}
              onChange={(event) => {
                const next = Number(event.target.value)
                if (!Number.isNaN(next)) {
                  onGitCacheTtlSecsChange(next)
                }
              }}
            />
          </div>
          <div className="settings-helper">{t('gitCacheTtlHint')}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-github-token">
            {t('githubToken')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-github-token"
              className="settings-input mono"
              type="password"
              placeholder={t('githubTokenPlaceholder')}
              value={localToken}
              onChange={(e) => setLocalToken(e.target.value)}
              onBlur={() => {
                if (localToken !== githubToken) {
                  onGithubTokenChange(localToken)
                }
              }}
            />
          </div>
          <div className="settings-helper">{t('githubTokenHint')}</div>
        </div>

        <div className="settings-field settings-update-section">
          <label className="settings-label">{t('appUpdates')}</label>
          <div className="settings-version-row">
            <span className="settings-version-text">
              {t('appName')} {versionText}
            </span>
            {isTauri && updateStatus === 'idle' && (
              <button
                className="btn btn-secondary btn-sm"
                type="button"
                onClick={handleCheckUpdate}
              >
                {t('checkForUpdates')}
              </button>
            )}
            {updateStatus === 'checking' && (
              <span className="settings-update-status">{t('checkingUpdates')}</span>
            )}
            {updateStatus === 'up-to-date' && (
              <span className="settings-update-status settings-update-ok">{t('updateNotAvailable')}</span>
            )}
          </div>
          {updateStatus === 'available' && (
            <div className="settings-update-available">
              <span>{t('updateAvailableWithVersion', { version: updateVersion })}</span>
              <button
                className="btn btn-primary btn-sm"
                type="button"
                onClick={handleInstallUpdate}
              >
                {t('downloadAndInstall')}
              </button>
            </div>
          )}
          {updateStatus === 'downloading' && (
            <div className="settings-update-status">{t('installingUpdate')}</div>
          )}
          {updateStatus === 'done' && (
            <div className="settings-update-ok">{t('updateInstalledRestart')}</div>
          )}
          {updateStatus === 'error' && (
            <div className="settings-update-error">
              <span>{updateError}</span>
              <button
                className="btn btn-secondary btn-sm"
                type="button"
                onClick={handleCheckUpdate}
              >
                {t('checkForUpdates')}
              </button>
            </div>
          )}
          <div className="settings-helper">{t('updateHint')}</div>
        </div>

        <div className="settings-field deploy-profiles-section">
          <div className="deploy-profiles-header-row">
            <label className="settings-label">{t('deployProfiles')}</label>
            <button
              className="btn btn-secondary btn-sm"
              type="button"
              onClick={() => {
                onCreateProfile(
                  `Profile ${deployProfiles.length + 1}`,
                  JSON.stringify({ skill: [], mcp_server: [], plugin: [] }),
                  deployProfiles.length === 0,
                )
              }}
            >
              {t('deployProfileAdd')}
            </button>
          </div>
          {deployProfiles.length === 0 ? (
            <div className="deploy-profile-empty">{t('deployProfileNoProfiles')}</div>
          ) : (
            deployProfiles.map((profile) => (
              <DeployProfileCard
                key={profile.id}
                profile={profile}
                profileCount={deployProfiles.length}
                onUpdate={onUpdateProfile}
                onDelete={onDeleteProfile}
                t={t}
              />
            ))
          )}
        </div>

      </div>
    </div>
  )
}

export default memo(SettingsPage)
