export type OnboardingVariant = {
  tool: string
  name: string
  path: string
  fingerprint?: string | null
  is_link: boolean
  link_target?: string | null
}

export type OnboardingGroup = {
  name: string
  variants: OnboardingVariant[]
  has_conflict: boolean
}

export type OnboardingPlan = {
  total_tools_scanned: number
  total_skills_found: number
  groups: OnboardingGroup[]
}

export type ToolOption = {
  id: string
  label: string
}

export type ManagedSkill = {
  id: string
  name: string
  description?: string | null
  source_type: string
  source_ref?: string | null
  central_path: string
  created_at: number
  updated_at: number
  last_sync_at?: number | null
  status: string
  asset_type: string
  config_json?: string | null
  security_status?: string | null
  targets: {
    tool: string
    sync_mode: string
    status: string
    target_path: string
    synced_at?: number | null
  }[]
}

export type GitSkillCandidate = {
  name: string
  description?: string | null
  subpath: string
}

export type LocalSkillCandidate = {
  name: string
  description?: string | null
  subpath: string
  valid: boolean
  reason?: string | null
}

export type InstallResultDto = {
  skill_id: string
  name: string
  central_path: string
  content_hash?: string | null
}

export type ToolInfoDto = {
  key: string
  label: string
  installed: boolean
  skills_dir: string
}

export type ToolStatusDto = {
  tools: ToolInfoDto[]
  installed: string[]
  newly_installed: string[]
}

export type UpdateResultDto = {
  skill_id: string
  name: string
  content_hash?: string | null
  source_revision?: string | null
  updated_targets: string[]
}

export type FeaturedSkillDto = {
  slug: string
  name: string
  summary: string
  downloads: number
  stars: number
  source_url: string
}

export type OnlineSkillDto = {
  name: string
  installs: number
  source: string
  source_url: string
}

export type SkillFileEntry = {
  path: string
  size: number
}

export type ScannedMcpServerDto = {
  tool_key: string
  name: string
  command: string
  args: string[]
  env: Record<string, string>
}

export type PluginInfoDto = {
  name: string
  marketplace: string
  version: string
  install_path: string
  installed_at: string
  scope: string
  git_commit_sha?: string | null
}

export type PluginHealthDto = {
  name: string
  healthy: boolean
  issues: string[]
}

export type MarketplaceInfoDto = {
  name: string
  plugin_count: number
  path: string
}

export type LibraryDto = {
  id: string
  name: string
  url: string
  library_type: string
  asset_types: string
  trusted: boolean
  last_indexed_at?: number | null
  item_count?: number | null
  status: string
}

export type LibraryItemDto = {
  id: string
  library_id: string
  asset_type: string
  name: string
  description?: string | null
  subpath?: string | null
  metadata_json?: string | null
  indexed_at: number
}
