import { ref, onScopeDispose } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface UploadFile {
  id: string
  name: string
  type: string
  size: number
  previewUrl?: string
  base64?: string
  filePath?: string
}

function getMimeType(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() || ''
  const map: Record<string, string> = {
    jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png',
    gif: 'image/gif', webp: 'image/webp', bmp: 'image/bmp',
    svg: 'image/svg+xml', pdf: 'application/pdf',
    txt: 'text/plain', md: 'text/markdown',
    json: 'application/json', csv: 'text/csv',
  }
  return map[ext] || 'application/octet-stream'
}

export function useFileUpload() {
  const files = ref<UploadFile[]>([])

  function generateId() {
    return Math.random().toString(36).slice(2, 10)
  }

  async function addFiles(fileList: FileList | File[]) {
    for (const file of Array.from(fileList)) {
      const id = generateId()
      const uploadFile: UploadFile = {
        id,
        name: file.name,
        type: file.type,
        size: file.size,
      }
      if (file.type.startsWith('image/')) {
        uploadFile.previewUrl = URL.createObjectURL(file)
      }
      const reader = new FileReader()
      const base64 = await new Promise<string>((resolve) => {
        reader.onload = () => {
          const result = reader.result as string
          resolve(result.split(',')[1] || '')
        }
        reader.onerror = () => {
          console.warn('FileReader 读取失败', file.name)
          resolve('')
        }
        reader.readAsDataURL(file)
      })
      uploadFile.base64 = base64
      files.value.push(uploadFile)
    }
  }

  async function addFilesFromPaths(paths: string[], includePath: boolean) {
    for (const filePath of paths) {
      const name = filePath.split(/[\\/]/).pop() || filePath
      const type = getMimeType(name)
      const id = generateId()
      const uploadFile: UploadFile = {
        id,
        name,
        type,
        size: 0,
        filePath: includePath ? filePath : undefined,
      }
      try {
        const base64: string = await invoke('read_file_base64', { path: filePath })
        uploadFile.base64 = base64
        uploadFile.size = Math.floor(base64.length * 3 / 4)
        if (type.startsWith('image/')) {
          uploadFile.previewUrl = `data:${type};base64,${base64}`
        }
      } catch (e) {
        console.warn('读取文件失败', filePath, e)
        continue
      }
      files.value.push(uploadFile)
    }
  }

  function removeFile(id: string) {
    const idx = files.value.findIndex((f) => f.id === id)
    if (idx !== -1) {
      const file = files.value[idx]
      if (file.previewUrl && !file.previewUrl.startsWith('data:')) {
        URL.revokeObjectURL(file.previewUrl)
      }
      files.value.splice(idx, 1)
    }
  }

  function clearFiles() {
    files.value.forEach((f) => {
      if (f.previewUrl && !f.previewUrl.startsWith('data:')) {
        URL.revokeObjectURL(f.previewUrl)
      }
    })
    files.value = []
  }

  onScopeDispose(() => clearFiles())

  return { files, addFiles, addFilesFromPaths, removeFile, clearFiles }
}
