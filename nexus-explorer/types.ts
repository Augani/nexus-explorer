export interface FileNode {
  id: string;
  name: string;
  type: 'file' | 'folder';
  content?: string;
  language?: string;
  children?: FileNode[];
  isOpen?: boolean;
  size?: string;
  date?: string;
  extension?: string;
}

export interface TerminalLine {
  id: string;
  type: 'input' | 'output' | 'info' | 'error' | 'success';
  content: string;
}