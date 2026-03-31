import { memo, useState, useCallback } from 'react'
import type { TFunction } from 'i18next'
import { Plus, Trash2 } from 'lucide-react'

type McpConfigModalProps = {
  open: boolean
  loading: boolean
  onRequestClose: () => void
  onSubmit: (name: string, command: string, args: string[], env: Record<string, string>) => void
  t: TFunction
}

const McpConfigModal = ({
  open,
  loading,
  onRequestClose,
  onSubmit,
  t,
}: McpConfigModalProps) => {
  const [name, setName] = useState('')
  const [command, setCommand] = useState('')
  const [argsText, setArgsText] = useState('')
  const [envRows, setEnvRows] = useState<{ key: string; value: string }[]>([])

  const handleAddEnvRow = useCallback(() => {
    setEnvRows((prev) => [...prev, { key: '', value: '' }])
  }, [])

  const handleRemoveEnvRow = useCallback((index: number) => {
    setEnvRows((prev) => prev.filter((_, i) => i !== index))
  }, [])

  const handleEnvChange = useCallback(
    (index: number, field: 'key' | 'value', val: string) => {
      setEnvRows((prev) =>
        prev.map((row, i) => (i === index ? { ...row, [field]: val } : row)),
      )
    },
    [],
  )

  const handleSubmit = useCallback(() => {
    if (!name.trim() || !command.trim()) return
    const args = argsText
      .split(',')
      .map((a) => a.trim())
      .filter((a) => a.length > 0)
    const env: Record<string, string> = {}
    for (const row of envRows) {
      if (row.key.trim()) {
        env[row.key.trim()] = row.value
      }
    }
    onSubmit(name.trim(), command.trim(), args, env)
    setName('')
    setCommand('')
    setArgsText('')
    setEnvRows([])
  }, [name, command, argsText, envRows, onSubmit])

  if (!open) return null

  return (
    <div className="modal-backdrop" onClick={loading ? undefined : onRequestClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="modal-title">{t('mcpAddServer')}</h2>

        <div className="mcp-form-group">
          <label className="form-label">{t('mcpServerName')}</label>
          <input
            type="text"
            className="form-input"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. my-mcp-server"
            disabled={loading}
          />
        </div>

        <div className="mcp-form-group">
          <label className="form-label">{t('mcpServerCommand')}</label>
          <input
            type="text"
            className="form-input"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            placeholder="e.g. npx"
            disabled={loading}
          />
        </div>

        <div className="mcp-form-group">
          <label className="form-label">{t('mcpServerArgs')}</label>
          <input
            type="text"
            className="form-input"
            value={argsText}
            onChange={(e) => setArgsText(e.target.value)}
            placeholder="e.g. -y, @modelcontextprotocol/server-name"
            disabled={loading}
          />
        </div>

        <div className="mcp-form-group">
          <label className="form-label">{t('mcpServerEnv')}</label>
          {envRows.map((row, index) => (
            <div className="mcp-env-row" key={index}>
              <input
                type="text"
                className="form-input"
                value={row.key}
                onChange={(e) => handleEnvChange(index, 'key', e.target.value)}
                placeholder={t('mcpEnvKey')}
                disabled={loading}
              />
              <input
                type="text"
                className="form-input"
                value={row.value}
                onChange={(e) => handleEnvChange(index, 'value', e.target.value)}
                placeholder={t('mcpEnvValue')}
                disabled={loading}
              />
              <button
                type="button"
                className="btn btn-icon btn-ghost"
                onClick={() => handleRemoveEnvRow(index)}
                disabled={loading}
              >
                <Trash2 size={14} />
              </button>
            </div>
          ))}
          <button
            type="button"
            className="mcp-env-add-btn"
            onClick={handleAddEnvRow}
            disabled={loading}
          >
            <Plus size={14} />
            {t('mcpAddEnvVar')}
          </button>
        </div>

        <div className="modal-actions">
          <button
            className="btn btn-secondary"
            type="button"
            onClick={onRequestClose}
            disabled={loading}
          >
            {t('cancel')}
          </button>
          <button
            className="btn btn-primary"
            type="button"
            onClick={handleSubmit}
            disabled={loading || !name.trim() || !command.trim()}
          >
            {t('create')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default memo(McpConfigModal)
