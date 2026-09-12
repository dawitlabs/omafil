import ArchiveIcon from '@fluentui/svg-icons/icons/folder_zip_20_filled.svg?no-inline'
import CodeIcon from '@fluentui/svg-icons/icons/code_20_filled.svg?no-inline'
import DocumentIcon from '@fluentui/svg-icons/icons/document_20_filled.svg?no-inline'
import FolderIcon from '@fluentui/svg-icons/icons/folder_20_filled.svg?no-inline'
import ImageIcon from '@fluentui/svg-icons/icons/image_20_filled.svg?no-inline'
import MusicIcon from '@fluentui/svg-icons/icons/music_note_2_20_filled.svg?no-inline'
import PdfIcon from '@fluentui/svg-icons/icons/document_pdf_20_filled.svg?no-inline'
import SlideIcon from '@fluentui/svg-icons/icons/slide_layout_20_filled.svg?no-inline'
import TableIcon from '@fluentui/svg-icons/icons/table_20_filled.svg?no-inline'
import TextIcon from '@fluentui/svg-icons/icons/document_text_20_filled.svg?no-inline'
import VideoIcon from '@fluentui/svg-icons/icons/video_20_filled.svg?no-inline'

export type FileIcon = {
  icon: string
  tone: string
  /** freedesktop icon name, used when an Omarchy icon theme is active. */
  themeName: string
}

const folderIcon: FileIcon = { icon: FolderIcon, tone: '#efa92a', themeName: 'folder' }
const genericIcon: FileIcon = { icon: DocumentIcon, tone: '#8a8a92', themeName: 'text-x-generic' }

const folderThemeNames: Record<string, string> = {
  documents: 'folder-documents', downloads: 'folder-download', music: 'folder-music',
  pictures: 'folder-pictures', videos: 'folder-videos', desktop: 'folder-desktop',
}

export function folderThemeName(folderName: string): string {
  return folderThemeNames[folderName.toLowerCase()] ?? 'folder'
}

const iconsByExtension: Record<string, FileIcon> = {}

function register(extensions: string[], icon: FileIcon) {
  for (const extension of extensions) {
    iconsByExtension[extension] = icon
  }
}

register(['png', 'jpg', 'jpeg', 'gif', 'webp', 'avif', 'bmp', 'svg', 'ico'], { icon: ImageIcon, tone: '#2b8fd6', themeName: 'image-x-generic' })
register(['mp3', 'wav', 'flac', 'm4a', 'ogg', 'opus', 'aac'], { icon: MusicIcon, tone: '#c94fb0', themeName: 'audio-x-generic' })
register(['mp4', 'mkv', 'mov', 'webm', 'avi', 'm4v'], { icon: VideoIcon, tone: '#e06c3b', themeName: 'video-x-generic' })
register(['doc', 'docx', 'odt', 'rtf'], { icon: DocumentIcon, tone: '#2b579a', themeName: 'x-office-document' })
register(['txt', 'md', 'log'], { icon: TextIcon, tone: '#6d6d75', themeName: 'text-x-generic' })
register(['xls', 'xlsx', 'ods', 'csv', 'tsv'], { icon: TableIcon, tone: '#217346', themeName: 'x-office-spreadsheet' })
register(['ppt', 'pptx', 'odp'], { icon: SlideIcon, tone: '#c43e1c', themeName: 'x-office-presentation' })
register(['pdf'], { icon: PdfIcon, tone: '#d13438', themeName: 'application-pdf' })
register(['zip', 'tar', 'gz', 'xz', 'zst', '7z', 'rar'], { icon: ArchiveIcon, tone: '#b58a1b', themeName: 'package-x-generic' })
register(
  ['ts', 'tsx', 'js', 'jsx', 'svelte', 'vue', 'rs', 'py', 'go', 'c', 'h', 'cpp', 'java', 'rb', 'sh', 'json', 'toml', 'yaml', 'yml', 'xml', 'xaml', 'html', 'css'],
  { icon: CodeIcon, tone: '#7a4fc0', themeName: 'text-x-script' },
)

export function fileIcon(name: string): FileIcon {
  const dotIndex = name.lastIndexOf('.')

  if (dotIndex <= 0) return folderIcon

  return iconsByExtension[name.slice(dotIndex + 1).toLowerCase()] ?? genericIcon
}

export { folderIcon }
