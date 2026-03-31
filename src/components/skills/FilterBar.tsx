import { memo } from 'react'
import { ArrowUpDown, RefreshCw, Search } from 'lucide-react'
import type { TFunction } from 'i18next'

type AssetTypeFilter = 'all' | 'skill' | 'mcp_server' | 'plugin' | 'executable'

type FilterBarProps = {
  sortBy: 'updated' | 'name'
  searchQuery: string
  loading: boolean
  assetTypeFilter: AssetTypeFilter
  onSortChange: (value: 'updated' | 'name') => void
  onSearchChange: (value: string) => void
  onAssetTypeFilterChange: (value: AssetTypeFilter) => void
  onRefresh: () => void
  t: TFunction
}

const ASSET_TYPE_FILTERS: AssetTypeFilter[] = ['all', 'skill', 'mcp_server', 'plugin', 'executable']

const FILTER_I18N_KEYS: Record<AssetTypeFilter, string> = {
  all: 'filterAll',
  skill: 'filterSkills',
  mcp_server: 'filterMcpServers',
  plugin: 'filterPlugins',
  executable: 'filterExecutables',
}

const FilterBar = ({
  sortBy,
  searchQuery,
  loading,
  assetTypeFilter,
  onSortChange,
  onSearchChange,
  onAssetTypeFilterChange,
  onRefresh,
  t,
}: FilterBarProps) => {
  return (
    <div className="filter-bar-wrapper">
      <div className="filter-bar">
        <div className="filter-title">{t('allSkills')}</div>
        <div className="filter-actions">
          <button className="btn btn-secondary sort-btn" type="button">
            <span className="sort-label">{t('filterSort')}:</span>
            {sortBy === 'updated' ? t('sortUpdated') : t('sortName')}
            <ArrowUpDown size={12} />
            <select
              aria-label={t('filterSort')}
              value={sortBy}
              onChange={(event) => onSortChange(event.target.value as 'updated' | 'name')}
            >
              <option value="updated">{t('sortUpdated')}</option>
              <option value="name">{t('sortName')}</option>
            </select>
          </button>
          <div className="search-container">
            <Search size={16} className="search-icon-abs" />
            <input
              className="search-input"
              value={searchQuery}
              onChange={(event) => onSearchChange(event.target.value)}
              placeholder={t('searchPlaceholder')}
            />
          </div>
          <button
            className="btn btn-secondary"
            type="button"
            onClick={onRefresh}
            disabled={loading}
          >
            <RefreshCw size={14} />
            {t('refresh')}
          </button>
        </div>
      </div>
      <div className="type-filter-bar">
        {ASSET_TYPE_FILTERS.map((filter) => (
          <button
            key={filter}
            type="button"
            className={`type-filter-chip${assetTypeFilter === filter ? ' active' : ''}`}
            onClick={() => onAssetTypeFilterChange(filter)}
          >
            {t(FILTER_I18N_KEYS[filter])}
          </button>
        ))}
      </div>
    </div>
  )
}

export default memo(FilterBar)
