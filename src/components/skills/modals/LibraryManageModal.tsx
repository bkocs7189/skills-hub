import { memo, useState } from 'react'
import { Trash2, ShieldCheck } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { LibraryDto } from '../types'

type LibraryManageModalProps = {
  open: boolean
  libraries: LibraryDto[]
  onRequestClose: () => void
  onAddLibrary: (name: string, url: string, libraryType: string) => void
  onDeleteLibrary: (libraryId: string) => void
  t: TFunction
}

const LibraryManageModal = ({
  open,
  libraries,
  onRequestClose,
  onAddLibrary,
  onDeleteLibrary,
  t,
}: LibraryManageModalProps) => {
  const [name, setName] = useState('')
  const [url, setUrl] = useState('')
  const [libraryType, setLibraryType] = useState('marketplace')

  if (!open) return null

  const handleAdd = () => {
    if (!name.trim() || !url.trim()) return
    onAddLibrary(name.trim(), url.trim(), libraryType)
    setName('')
    setUrl('')
    setLibraryType('marketplace')
  }

  return (
    <div className="modal-backdrop" onClick={onRequestClose}>
      <div
        className="modal modal-lg"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="modal-header">
          <div className="modal-title">{t('libraryManage')}</div>
          <button
            className="btn btn-secondary"
            onClick={onRequestClose}
            type="button"
          >
            {t('close')}
          </button>
        </div>
        <div className="modal-body">
          <div className="library-manage-list">
            {libraries.length === 0 ? (
              <div className="explore-empty">{t('exploreEmpty')}</div>
            ) : (
              libraries.map((lib) => (
                <div key={lib.id} className="library-manage-item">
                  <div className="library-manage-info">
                    <div className="library-manage-name">
                      {lib.name}
                      {lib.trusted && (
                        <span className="library-trusted-badge">
                          <ShieldCheck size={12} />
                          {t('libraryTrusted')}
                        </span>
                      )}
                    </div>
                    <div className="library-manage-url">{lib.url}</div>
                    <div className="library-manage-meta">
                      {lib.library_type === 'marketplace'
                        ? t('libraryTypeMarketplace')
                        : lib.library_type === 'github_repo'
                          ? t('libraryTypeGithub')
                          : t('libraryTypeCurated')}
                      {lib.item_count != null && lib.item_count > 0 && (
                        <span> &middot; {t('libraryItemCount', { count: lib.item_count })}</span>
                      )}
                    </div>
                  </div>
                  <button
                    className="btn btn-danger-ghost library-manage-delete"
                    type="button"
                    onClick={() => onDeleteLibrary(lib.id)}
                    title={t('libraryDelete')}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))
            )}
          </div>

          <div className="library-add-form">
            <div className="library-add-title">{t('libraryAdd')}</div>
            <div className="library-add-fields">
              <input
                className="form-input"
                placeholder={t('libraryName')}
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
              <input
                className="form-input"
                placeholder={t('libraryUrl')}
                value={url}
                onChange={(e) => setUrl(e.target.value)}
              />
              <select
                className="form-input library-type-select"
                value={libraryType}
                onChange={(e) => setLibraryType(e.target.value)}
              >
                <option value="marketplace">{t('libraryTypeMarketplace')}</option>
                <option value="github_repo">{t('libraryTypeGithub')}</option>
                <option value="curated_list">{t('libraryTypeCurated')}</option>
              </select>
              <button
                className="btn btn-primary"
                type="button"
                onClick={handleAdd}
                disabled={!name.trim() || !url.trim()}
              >
                {t('libraryAdd')}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default memo(LibraryManageModal)
