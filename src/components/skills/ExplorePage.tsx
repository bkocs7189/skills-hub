import { memo, useMemo } from 'react'
import { Library, Plus, Search, Settings2, ShieldCheck, Star } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { FeaturedSkillDto, LibraryDto, LibraryItemDto, ManagedSkill, OnlineSkillDto } from './types'

type ExplorePageProps = {
  featuredSkills: FeaturedSkillDto[]
  featuredLoading: boolean
  exploreFilter: string
  searchResults: OnlineSkillDto[]
  searchLoading: boolean
  managedSkills: ManagedSkill[]
  loading: boolean
  libraries: LibraryDto[]
  libraryItems: LibraryItemDto[]
  selectedLibraryId: string
  librarySearchQuery: string
  librarySearchResults: LibraryItemDto[]
  onExploreFilterChange: (value: string) => void
  onInstallSkill: (sourceUrl: string, skillName?: string) => void
  onOpenManualAdd: () => void
  onOpenLibraryManage: () => void
  onLibrarySelect: (libraryId: string) => void
  onLibrarySearch: (query: string) => void
  t: TFunction
}

function formatCount(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`
  return String(n)
}

const ExplorePage = ({
  featuredSkills,
  featuredLoading,
  exploreFilter,
  searchResults,
  searchLoading,
  managedSkills,
  loading,
  libraries,
  libraryItems,
  selectedLibraryId,
  librarySearchQuery,
  librarySearchResults,
  onExploreFilterChange,
  onInstallSkill,
  onOpenManualAdd,
  onOpenLibraryManage,
  onLibrarySelect,
  onLibrarySearch,
  t,
}: ExplorePageProps) => {
  const filteredSkills = useMemo(() => {
    if (!exploreFilter.trim()) return featuredSkills
    const lower = exploreFilter.toLowerCase()
    return featuredSkills.filter(
      (s) =>
        s.name.toLowerCase().includes(lower) ||
        s.summary.toLowerCase().includes(lower),
    )
  }, [featuredSkills, exploreFilter])

  const deduplicatedResults = useMemo(() => {
    const featuredNames = new Set(filteredSkills.map((s) => s.name.toLowerCase()))
    return searchResults.filter((s) => !featuredNames.has(s.name.toLowerCase()))
  }, [searchResults, filteredSkills])

  const isSearchActive = exploreFilter.trim().length >= 2

  const installedSkillKeys = useMemo(() => {
    const keys = new Set<string>()
    for (const skill of managedSkills) {
      const source = (skill.source_ref ?? '')
        .replace('https://github.com/', '')
        .replace(/\.git$/, '')
        .split('/tree/')[0]
        .toLowerCase()
      keys.add(`${skill.name.toLowerCase()}|${source}`)
    }
    return keys
  }, [managedSkills])

  const isInstalled = (skillName: string, source: string) => {
    const normalizedSource = source
      .replace('https://github.com/', '')
      .replace(/\.git$/, '')
      .split('/tree/')[0]
      .toLowerCase()
    return installedSkillKeys.has(`${skillName.toLowerCase()}|${normalizedSource}`)
  }

  const displayLibraryItems = librarySearchQuery.trim().length >= 2
    ? librarySearchResults
    : libraryItems

  return (
    <div className="explore-page">
      <div className="explore-hero">
        <div className="explore-search-row">
          <div className="explore-search-wrap">
            <Search size={16} className="explore-search-icon" />
            <input
              className="explore-search-input"
              placeholder={t('exploreFilterPlaceholder')}
              value={exploreFilter}
              onChange={(e) => onExploreFilterChange(e.target.value)}
            />
          </div>
          <button
            className="btn btn-secondary explore-manual-btn"
            type="button"
            onClick={onOpenManualAdd}
            disabled={loading}
          >
            <Plus size={15} />
            {t('manualAdd')}
          </button>
        </div>

        {/* Library selector row */}
        {libraries.length > 0 && (
          <div className="library-selector-row">
            <div className="library-selector-left">
              <Library size={14} />
              <select
                className="library-selector"
                value={selectedLibraryId}
                onChange={(e) => onLibrarySelect(e.target.value)}
              >
                <option value="">{t('librariesTitle')}</option>
                {libraries.map((lib) => (
                  <option key={lib.id} value={lib.id}>
                    {lib.name}
                    {lib.trusted ? ` (${t('libraryTrusted')})` : ''}
                  </option>
                ))}
              </select>
              {selectedLibraryId && (
                <div className="library-search-wrap">
                  <Search size={13} className="library-search-icon" />
                  <input
                    className="library-search-input"
                    placeholder={t('librarySearch')}
                    value={librarySearchQuery}
                    onChange={(e) => onLibrarySearch(e.target.value)}
                  />
                </div>
              )}
            </div>
            <button
              className="btn btn-ghost library-manage-btn"
              type="button"
              onClick={onOpenLibraryManage}
              title={t('libraryManage')}
            >
              <Settings2 size={14} />
              {t('libraryManage')}
            </button>
          </div>
        )}

        <div className="explore-source-label">
          {t('exploreSourceHint')}
        </div>
      </div>

      <div className="explore-scroll">
        {/* Library items section */}
        {selectedLibraryId && displayLibraryItems.length > 0 && (
          <>
            <div className="explore-section-title">
              {t('librariesTitle')}
              {libraries.find((l) => l.id === selectedLibraryId)?.trusted && (
                <span className="library-trusted-badge">
                  <ShieldCheck size={12} />
                  {t('libraryTrusted')}
                </span>
              )}
            </div>
            <div className="explore-grid">
              {displayLibraryItems.map((item) => (
                <div key={item.id} className="explore-card">
                  <div className="explore-card-top">
                    <div className="explore-card-info">
                      <div className="explore-card-name">{item.name}</div>
                      <div className="explore-card-author">{item.asset_type}</div>
                    </div>
                    <button
                      className="explore-btn-install"
                      type="button"
                      disabled={loading}
                      onClick={() => {
                        const lib = libraries.find((l) => l.id === item.library_id)
                        if (lib) {
                          const sourceUrl = item.subpath
                            ? `${lib.url}/tree/main/${item.subpath}`
                            : lib.url
                          onInstallSkill(sourceUrl, item.name)
                        }
                      }}
                    >
                      {t('install')}
                    </button>
                  </div>
                  {item.description && (
                    <div className="explore-card-desc">{item.description}</div>
                  )}
                </div>
              ))}
            </div>
          </>
        )}

        {/* Featured section */}
        {featuredLoading ? (
          <div className="explore-loading">{t('exploreLoading')}</div>
        ) : (
          <>
            {isSearchActive && filteredSkills.length > 0 && (
              <div className="explore-section-title">{t('exploreFeaturedTitle')}</div>
            )}
            {filteredSkills.length > 0 ? (
              <div className="explore-grid">
                {filteredSkills.map((skill) => {
                  const installed = isInstalled(skill.name, skill.source_url)
                  return (
                    <div key={skill.slug} className="explore-card">
                      <div className="explore-card-top">
                        <div className="explore-card-info">
                          <div className="explore-card-name">{skill.name}</div>
                          <div className="explore-card-author">
                            {skill.source_url
                              .replace('https://github.com/', '')
                              .split('/tree/')[0]}
                          </div>
                        </div>
                        {installed ? (
                          <span className="explore-btn-installed">
                            {t('status.installed')}
                          </span>
                        ) : (
                          <button
                            className="explore-btn-install"
                            type="button"
                            disabled={loading}
                            onClick={() => onInstallSkill(skill.source_url)}
                          >
                            {t('install')}
                          </button>
                        )}
                      </div>
                      <div className="explore-card-desc">{skill.summary}</div>
                      <div className="explore-card-bottom">
                        <div className="explore-card-stats">
                          <span className="explore-stat">
                            <Star size={12} />
                            {formatCount(skill.stars)}
                          </span>
                        </div>
                      </div>
                    </div>
                  )
                })}
              </div>
            ) : !isSearchActive ? (
              <div className="explore-empty">{t('exploreEmpty')}</div>
            ) : null}

            {/* Online search results */}
            {isSearchActive && (
              <>
                <div className="explore-section-title">{t('exploreOnlineTitle')}</div>
                {searchLoading ? (
                  <div className="explore-loading">{t('searchLoading')}</div>
                ) : deduplicatedResults.length > 0 ? (
                  <div className="explore-grid">
                    {deduplicatedResults.map((skill) => {
                      const installed = isInstalled(skill.name, skill.source_url)
                      return (
                        <div key={skill.source} className="explore-card">
                          <div className="explore-card-top">
                            <div className="explore-card-info">
                              <div className="explore-card-name">{skill.name}</div>
                              <div className="explore-card-author">{skill.source}</div>
                            </div>
                            {installed ? (
                              <span className="explore-btn-installed">
                                {t('status.installed')}
                              </span>
                            ) : (
                              <button
                                className="explore-btn-install"
                                type="button"
                                disabled={loading}
                                onClick={() => onInstallSkill(skill.source_url, skill.name)}
                              >
                                {t('install')}
                              </button>
                            )}
                          </div>
                          <div className="explore-card-bottom">
                            <div className="explore-card-stats">
                              <span className="explore-stat">
                                {formatCount(skill.installs)} installs
                              </span>
                            </div>
                          </div>
                        </div>
                      )
                    })}
                  </div>
                ) : (
                  <div className="explore-empty">{t('searchEmpty')}</div>
                )}
              </>
            )}
          </>
        )}
      </div>
    </div>
  )
}

export default memo(ExplorePage)
