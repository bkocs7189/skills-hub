import { memo } from 'react'
import type { TFunction } from 'i18next'
import type { SecurityResultDto } from '../types'

type SecurityScanModalProps = {
  open: boolean
  loading: boolean
  result: SecurityResultDto | null
  onRequestClose: () => void
  onDeepScan: () => void
  hasDeepScanned: boolean
  t: TFunction
}

const SecurityScanModal = ({
  open,
  loading,
  result,
  onRequestClose,
  onDeepScan,
  hasDeepScanned,
  t,
}: SecurityScanModalProps) => {
  if (!open) return null

  const statusLabel = result
    ? result.status === 'trusted'
      ? t('securityTrusted')
      : result.status === 'flagged'
        ? t('securityFlagged')
        : t('securityUnknown')
    : ''

  return (
    <div className="modal-backdrop" onClick={onRequestClose}>
      <div
        className="modal-card"
        style={{ width: 420, maxHeight: '80vh', overflow: 'auto' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-title">{t('securityTrusted', { defaultValue: 'Security Check' })}</div>

        {loading && !result && (
          <div style={{ padding: '24px 0', textAlign: 'center', color: 'var(--text-secondary)' }}>
            {t('checkingUpdates', { defaultValue: 'Checking...' })}
          </div>
        )}

        {result && (
          <div style={{ padding: '8px 0' }}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>{statusLabel}</div>
            {result.findings.length === 0 ? (
              <div style={{ color: 'var(--text-secondary)', fontSize: 13 }}>
                {t('securityTrusted', { defaultValue: 'No issues found.' })}
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                {result.findings.map((finding: SecurityResultDto['findings'][number], idx: number) => (
                  <div
                    key={idx}
                    style={{
                      fontSize: 13,
                      padding: '6px 8px',
                      borderRadius: 4,
                      background: finding.severity === 'high'
                        ? 'color-mix(in srgb, var(--danger, #ef4444) 10%, transparent)'
                        : 'var(--bg-card, var(--bg-app))',
                      border: '1px solid var(--border-subtle)',
                    }}
                  >
                    <span style={{ fontWeight: 600, textTransform: 'uppercase', fontSize: 11 }}>
                      {finding.severity}
                    </span>
                    {' '}
                    <span>{finding.description}</span>
                    {finding.file_path && (
                      <span style={{ color: 'var(--text-secondary)', marginLeft: 4 }}>
                        ({finding.file_path})
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}

            {!hasDeepScanned && (
              <button
                className="btn btn-secondary btn-sm"
                type="button"
                style={{ marginTop: 12 }}
                onClick={onDeepScan}
                disabled={loading}
              >
                {loading
                  ? t('checkingUpdates', { defaultValue: 'Scanning...' })
                  : t('pluginDiagnose', { defaultValue: 'Deep Scan' })}
              </button>
            )}
          </div>
        )}

        <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 16 }}>
          <button className="btn btn-secondary" type="button" onClick={onRequestClose}>
            {t('close')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default memo(SecurityScanModal)
