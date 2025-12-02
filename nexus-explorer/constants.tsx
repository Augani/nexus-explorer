import { FileNode } from './types';

export const MOCK_FILE_SYSTEM: FileNode[] = [
  {
    id: 'root',
    name: 'Projects',
    type: 'folder',
    isOpen: true,
    children: [
      {
        id: 'react-app',
        name: 'React-App',
        type: 'folder',
        isOpen: true,
        size: '--',
        date: 'Oct 24, 2023',
        extension: 'Folder',
        children: [
          {
            id: 'node_modules',
            name: 'node_modules',
            type: 'folder',
            isOpen: false,
            size: '145 MB',
            date: 'Oct 24, 2023',
            extension: 'Folder',
            children: []
          },
          {
            id: 'src',
            name: 'src',
            type: 'folder',
            isOpen: false,
            size: '--',
            date: 'Oct 24, 2023',
            extension: 'Folder',
            children: [
              { 
                id: 'app-tsx', 
                name: 'App.tsx', 
                type: 'file', 
                language: 'typescript', 
                size: '2.4 KB',
                date: 'Just now',
                extension: 'TSX',
                content: '// Main App Component\nexport default function App() {\n  return <div>Hello World</div>\n}' 
              },
              { 
                id: 'index-tsx', 
                name: 'index.tsx', 
                type: 'file', 
                language: 'typescript', 
                size: '1.1 KB',
                date: '2 hours ago',
                extension: 'TSX',
                content: "import { createRoot } from 'react-dom/client';" 
              },
              { 
                id: 'components-folder', 
                name: 'components', 
                type: 'folder', 
                size: '--',
                date: 'Yesterday',
                extension: 'Folder',
                children: [] 
              },
              { 
                id: 'utils-ts', 
                name: 'utils.ts', 
                type: 'file', 
                language: 'typescript', 
                size: '4.2 KB',
                date: '3 days ago',
                extension: 'TS',
                content: "export const formatDate = (date: Date) => {\n  return new Intl.DateTimeFormat('en-US').format(date);\n}" 
              }
            ]
          },
          {
            id: 'gitignore',
            name: '.gitignore',
            type: 'file',
            language: 'plaintext',
            size: '128 B',
            date: 'Oct 24, 2023',
            extension: 'Config',
            content: "node_modules\n.env\ndist"
          },
          {
            id: 'package-json',
            name: 'package.json',
            type: 'file',
            language: 'json',
            size: '1.2 KB',
            date: 'Oct 25, 2023',
            extension: 'JSON',
            content: `{
  "name": "react-app",
  "version": "1.0.0",
  "private": true,
  "scripts": {
    "start": "react-scripts start",
    "build": "react-scripts build",
    "test": "react-scripts test"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-scripts": "5.0.1",
    "framer-motion": "^10.0.0"
  },
  "devDependencies": {
    "typescript": "^4.9.5",
    "tailwindcss": "^3.3.0"
  },
  "gitDependencies": {
    "listen-net": "^1.2.3"
  }
}`
          },
          {
            id: 'readme',
            name: 'README.md',
            type: 'file',
            language: 'markdown',
            size: '4.5 KB',
            date: 'Oct 24, 2023',
            extension: 'Markdown',
            content: "# React App\n\nThis is a sample project."
          },
          {
            id: 'tsconfig',
            name: 'tsconfig.json',
            type: 'file',
            language: 'json',
            size: '8.2 KB',
            date: 'Oct 24, 2023',
            extension: 'JSON',
            content: "{\n  \"compilerOptions\": {\n    \"target\": \"es5\"\n  }\n}"
          }
        ]
      },
      {
        id: 'backup-folder',
        name: 'Backups',
        type: 'folder',
        isOpen: false,
        size: '2.4 GB',
        date: 'Sep 12, 2023',
        extension: 'Folder',
        children: []
      }
    ]
  }
];

export const INITIAL_TERMINAL_LINES = [
  { id: '1', type: 'input', content: 'git status' },
  { id: '2', type: 'info', content: 'On branch main' },
  { id: '3', type: 'info', content: 'Your branch is up to date with \'origin/main\'.' },
  { id: '4', type: 'output', content: 'Changes not staged for commit:' },
  { id: '5', type: 'output', content: '  (use "git add <file>..." to update what will be committed)' },
  { id: '6', type: 'output', content: '  (use "git restore <file>..." to discard changes in working directory)' },
  { id: '7', type: 'error', content: '        modified:   package.json' },
  { id: '8', type: 'output', content: '' },
  { id: '9', type: 'output', content: 'no changes added to commit (use "git add" and/or "git commit -a")' },
];