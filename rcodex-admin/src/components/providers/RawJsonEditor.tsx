import { useState } from "react"
import { RawJsonModal } from "./RawJsonModal"

interface RawJsonEditorProps {
  showRawJson: boolean
  rawJsonValue: string
  setRawJsonValue: (value: string) => void
  setShowRawJson: (show: boolean) => void
  onSave: () => Promise<void>
}

export function RawJsonEditor({
  showRawJson,
  rawJsonValue,
  setRawJsonValue,
  setShowRawJson,
  onSave,
}: RawJsonEditorProps) {
  const [isSaving, setIsSaving] = useState(false)

  async function handleSave() {
    setIsSaving(true)
    try {
      await onSave()
    } finally {
      setIsSaving(false)
    }
  }

  if (!showRawJson) return null

  return (
    <RawJsonModal
      value={rawJsonValue}
      onChange={setRawJsonValue}
      onCancel={() => setShowRawJson(false)}
      onSave={handleSave}
      isSaving={isSaving}
    />
  )
}
