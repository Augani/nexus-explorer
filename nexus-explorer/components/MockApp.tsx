import React, { useState, useRef, useEffect, useMemo } from 'react';
import { 
  Folder, FileCode, FileJson, FileText, ChevronRight, ChevronDown, 
  Search, Monitor, HardDrive, Home, Cloud, 
  Terminal as TerminalIcon, X, Sparkles,
  Copy, Trash2, Grid, List as ListIcon,
  Maximize2, ArrowLeft, RefreshCw, ChevronUp, Calendar, ArrowDownUp
} from 'lucide-react';
import { FileNode, TerminalLine } from '../types';
import { MOCK_FILE_SYSTEM, INITIAL_TERMINAL_LINES } from '../constants';
import { analyzeCode } from '../services/geminiService';

// --- Helper Components ---

const FileIcon = ({ name, type, size = "sm" }: { name: string; type: 'file' | 'folder', size?: "sm" | "lg" }) => {
  const iconSize = size === "lg" ? "w-8 h-8" : "w-4 h-4";
  
  if (type === 'folder') return <Folder className={`${iconSize} text-blue-500 fill-blue-500/20`} />;
  if (name.endsWith('.json')) return <FileJson className={`${iconSize} text-yellow-400`} />;
  if (name.endsWith('.tsx') || name.endsWith('.ts')) return <FileCode className={`${iconSize} text-blue-400`} />;
  if (name.endsWith('.md')) return <FileText className={`${iconSize} text-gray-400`} />;
  return <FileText className={`${iconSize} text-gray-500`} />;
};

const SyntaxHighlight = ({ code, language }: { code: string; language: string }) => {
  if (language === 'json') {
    const lines = code.split('\n');
    return (
      <div className="font-mono text-xs md:text-sm leading-6 text-left">
        {lines.map((line, i) => {
          const parts = line.split(/(".*?"):/);
          return (
            <div key={i} className="whitespace-pre">
              <span className="text-gray-700 select-none mr-4 w-6 inline-block text-right">{i + 1}</span>
              {parts.map((part, j) => {
                if (j === 1) return <span key={j} className="text-green-400 font-semibold">{part}:</span>;
                if (part.trim().startsWith('"')) return <span key={j} className="text-orange-300">{part}</span>;
                if (part.trim().match(/true|false/)) return <span key={j} className="text-blue-400">{part}</span>;
                if (part.trim().match(/^\d/)) return <span key={j} className="text-purple-400">{part}</span>;
                return <span key={j} className="text-gray-300">{part}</span>;
              })}
            </div>
          );
        })}
      </div>
    );
  }
  return (
    <pre className="font-mono text-xs md:text-sm text-gray-300 p-4 leading-relaxed text-left">{code}</pre>
  );
};

// --- Main Mock App ---

type SortKey = 'name' | 'date' | 'extension' | 'size';
type SortDirection = 'asc' | 'desc';

export const MockApp: React.FC = () => {
  // Navigation State
  const [currentFolder, setCurrentFolder] = useState<FileNode>(
    MOCK_FILE_SYSTEM[0].children![0] // Default to 'React-App'
  );
  const [selectedFile, setSelectedFile] = useState<FileNode | null>(
    MOCK_FILE_SYSTEM[0].children![0].children![3] // Default to package.json
  );
  
  // UI State
  const [isTerminalOpen, setIsTerminalOpen] = useState(true);
  const [terminalLines, setTerminalLines] = useState<TerminalLine[]>(INITIAL_TERMINAL_LINES as any);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('list');
  const [sortConfig, setSortConfig] = useState<{ key: SortKey; direction: SortDirection }>({ 
    key: 'name', 
    direction: 'asc' 
  });
  
  const terminalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [terminalLines]);

  // Derived sorted files
  const sortedFiles = useMemo(() => {
    if (!currentFolder.children) return [];
    
    return [...currentFolder.children].sort((a, b) => {
      const aValue = a[sortConfig.key] || '';
      const bValue = b[sortConfig.key] || '';
      
      if (aValue < bValue) return sortConfig.direction === 'asc' ? -1 : 1;
      if (aValue > bValue) return sortConfig.direction === 'asc' ? 1 : -1;
      return 0;
    });
  }, [currentFolder, sortConfig]);

  const handleSort = (key: SortKey) => {
    setSortConfig(current => ({
      key,
      direction: current.key === key && current.direction === 'asc' ? 'desc' : 'asc'
    }));
  };

  const handleAiAnalyze = async () => {
    if (!selectedFile || !selectedFile.content) return;
    
    // Open terminal if closed to show output
    if (!isTerminalOpen) setIsTerminalOpen(true);
    
    setIsAnalyzing(true);
    setTerminalLines(prev => [...prev, 
      { id: Date.now().toString(), type: 'input', content: `nexus-ai analyze ${selectedFile.name}` },
      { id: Date.now().toString() + 'b', type: 'info', content: 'Analyzing file with Gemini...' }
    ]);

    const result = await analyzeCode(selectedFile.content, selectedFile.name);

    setTerminalLines(prev => [...prev, 
      { id: Date.now().toString() + 'c', type: 'success', content: `Analysis Result:\n${result}` },
      { id: Date.now().toString() + 'd', type: 'output', content: '' }
    ]);
    setIsAnalyzing(false);
  };

  const handleFolderClick = (node: FileNode) => {
    setCurrentFolder(node);
    setSelectedFile(null);
  };

  const handleFileClick = (node: FileNode) => {
    setSelectedFile(node);
  };

  const renderSidebarItem = (node: FileNode, depth = 0) => {
    const isSelected = currentFolder.id === node.id;
    const isFileSelected = selectedFile?.id === node.id;
    
    return (
      <div key={node.id}>
        <div 
          className={`flex items-center py-1.5 px-3 cursor-pointer select-none transition-colors duration-150 text-sm
            ${isSelected ? 'bg-blue-600/20 text-blue-100' : 'text-gray-400 hover:bg-gray-800 hover:text-gray-200'}
            ${isFileSelected ? 'bg-gray-800 text-white' : ''}
          `}
          style={{ paddingLeft: `${depth * 16 + 12}px` }}
          onClick={() => node.type === 'folder' ? handleFolderClick(node) : handleFileClick(node)}
        >
          <span className="mr-2 opacity-70">
            {node.type === 'folder' && (
              node.isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />
            )}
             {node.type === 'file' && <div className="w-[14px]" />}
          </span>
          <span className="mr-2"><FileIcon name={node.name} type={node.type} /></span>
          <span className="truncate">{node.name}</span>
        </div>
        {node.children && node.isOpen && (
          <div>
            {node.children.map(child => renderSidebarItem(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  const SortIcon = ({ column }: { column: SortKey }) => {
    if (sortConfig.key !== column) return <ArrowDownUp size={12} className="opacity-0 group-hover:opacity-30 ml-1" />;
    return sortConfig.direction === 'asc' 
      ? <ChevronUp size={12} className="text-blue-400 ml-1" />
      : <ChevronDown size={12} className="text-blue-400 ml-1" />;
  };

  return (
    <div className="w-full h-full bg-[#0d1117] rounded-xl overflow-hidden border border-gray-800 shadow-2xl flex flex-col font-sans relative text-left">
      
      {/* Title Bar */}
      <div className="h-10 bg-[#010409] flex items-center justify-between px-4 border-b border-gray-800 shrink-0">
        <div className="flex items-center space-x-3">
          <div className="flex space-x-1.5 mr-4">
            <div className="w-3 h-3 rounded-full bg-[#ff5f56] border border-[#e0443e]" />
            <div className="w-3 h-3 rounded-full bg-[#ffbd2e] border border-[#dea123]" />
            <div className="w-3 h-3 rounded-full bg-[#27c93f] border border-[#1aab29]" />
          </div>
          <div className="text-gray-400 text-xs font-medium flex items-center">
            <HardDrive size={12} className="mr-2" />
            Nexus Explorer
          </div>
        </div>
        
        {/* Universal Search */}
        <div className="relative w-1/3 max-w-md hidden md:block">
           <Search size={13} className="absolute left-3 top-1.5 text-gray-500" />
           <input 
             type="text" 
             placeholder="Search files, commands, and more..." 
             className="w-full bg-[#161b22] text-gray-300 text-xs rounded-md border border-gray-700 py-1.5 pl-9 pr-3 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 placeholder-gray-600"
           />
        </div>

        <div className="flex items-center space-x-3 text-gray-500">
           <Sparkles size={14} className="hover:text-yellow-400 cursor-pointer" />
           <Monitor size={14} className="hover:text-blue-400 cursor-pointer" />
        </div>
      </div>

      <div className="flex flex-1 overflow-hidden">
        
        {/* COLUMN 1: SIDEBAR */}
        <div className="w-64 bg-[#0d1117] border-r border-gray-800 flex flex-col shrink-0">
          <div className="p-3">
             <div className="text-[10px] font-bold text-gray-500 mb-2 px-2 uppercase tracking-wider">Favorites</div>
             <div className="space-y-0.5 mb-6">
                {[
                  { icon: Home, label: 'Home' },
                  { icon: Monitor, label: 'Desktop' },
                  { icon: FileText, label: 'Documents' },
                  { icon: Cloud, label: 'iCloud' },
                ].map((item, i) => (
                  <div key={i} className="flex items-center px-2 py-1.5 text-gray-400 hover:bg-gray-800 rounded-md cursor-pointer text-sm transition-colors group">
                    <item.icon size={14} className="mr-3 text-gray-500 group-hover:text-blue-400" /> {item.label}
                  </div>
                ))}
             </div>
             
             <div className="text-[10px] font-bold text-gray-500 mb-2 px-2 uppercase tracking-wider flex justify-between group cursor-pointer">
                <span>Workspace</span>
             </div>
          </div>
          <div className="overflow-y-auto flex-1 pb-4 custom-scrollbar">
             {MOCK_FILE_SYSTEM.map(node => renderSidebarItem(node))}
          </div>
        </div>

        {/* COLUMN 2: BROWSER (Main Content) */}
        <div className="flex-1 flex flex-col bg-[#010409] relative min-w-0">
          
          {/* Middle Column Toolbar */}
          <div className="h-12 bg-[#0d1117] border-b border-gray-800 flex items-center justify-between px-4 shrink-0">
             <div className="flex items-center space-x-2 overflow-hidden">
                <button className="p-1.5 rounded-md hover:bg-gray-800 text-gray-400 disabled:opacity-30">
                  <ArrowLeft size={16} />
                </button>
                <div className="h-4 w-px bg-gray-700 mx-1"></div>
                <div className="flex items-center text-sm text-gray-300 font-medium truncate">
                   <span className="text-gray-500 mr-1">Projects /</span> {currentFolder.name}
                </div>
             </div>
             
             <div className="flex items-center space-x-1">
                <button 
                  onClick={() => setIsTerminalOpen(!isTerminalOpen)}
                  className={`p-1.5 rounded-md transition-colors ${isTerminalOpen ? 'bg-blue-500/20 text-blue-400' : 'hover:bg-gray-800 text-gray-400'}`} 
                  title="Toggle Terminal"
                >
                  <TerminalIcon size={16} />
                </button>
                <div className="h-4 w-px bg-gray-700 mx-2"></div>
                <button className="p-1.5 rounded-md hover:bg-gray-800 text-gray-400" title="Copy">
                  <Copy size={16} />
                </button>
                <button className="p-1.5 rounded-md hover:bg-gray-800 text-gray-400" title="Trash">
                  <Trash2 size={16} />
                </button>
                <div className="h-4 w-px bg-gray-700 mx-2"></div>
                <div className="flex bg-gray-800 rounded-lg p-0.5">
                   <button 
                    onClick={() => setViewMode('grid')}
                    className={`p-1 rounded ${viewMode === 'grid' ? 'bg-gray-700 text-white shadow-sm' : 'text-gray-400'}`}
                   >
                     <Grid size={14} />
                   </button>
                   <button 
                    onClick={() => setViewMode('list')}
                    className={`p-1 rounded ${viewMode === 'list' ? 'bg-gray-700 text-white shadow-sm' : 'text-gray-400'}`}
                   >
                     <ListIcon size={14} />
                   </button>
                </div>
             </div>
          </div>

          {/* File Grid/List View */}
          <div className="flex-1 overflow-y-auto custom-scrollbar bg-[#010409]">
             {sortedFiles.length > 0 ? (
               <>
                 {viewMode === 'list' ? (
                   <div className="min-w-full inline-block align-middle">
                     {/* List Header */}
                     <div className="sticky top-0 bg-[#0d1117] border-b border-gray-800 z-10 grid grid-cols-[minmax(200px,1fr)_120px_100px_80px] text-xs font-semibold text-gray-500 uppercase tracking-wider select-none">
                       <div className="px-4 py-2 flex items-center cursor-pointer hover:bg-gray-800 hover:text-gray-300 group" onClick={() => handleSort('name')}>
                         Name <SortIcon column="name" />
                       </div>
                       <div className="px-4 py-2 flex items-center cursor-pointer hover:bg-gray-800 hover:text-gray-300 group border-l border-gray-800/50" onClick={() => handleSort('date')}>
                         Date Added <SortIcon column="date" />
                       </div>
                       <div className="px-4 py-2 flex items-center cursor-pointer hover:bg-gray-800 hover:text-gray-300 group border-l border-gray-800/50" onClick={() => handleSort('extension')}>
                         Type <SortIcon column="extension" />
                       </div>
                       <div className="px-4 py-2 flex items-center cursor-pointer hover:bg-gray-800 hover:text-gray-300 group border-l border-gray-800/50" onClick={() => handleSort('size')}>
                         Size <SortIcon column="size" />
                       </div>
                     </div>
                     
                     {/* List Rows */}
                     <div className="divide-y divide-gray-800/50">
                       {sortedFiles.map(child => (
                         <div 
                           key={child.id}
                           onClick={() => child.type === 'folder' ? handleFolderClick(child) : handleFileClick(child)}
                           className={`
                             grid grid-cols-[minmax(200px,1fr)_120px_100px_80px] text-sm items-center cursor-pointer group
                             ${selectedFile?.id === child.id ? 'bg-blue-900/20' : 'hover:bg-gray-800/50'}
                           `}
                         >
                           <div className="px-4 py-2 flex items-center truncate">
                             <div className="mr-3 shrink-0"><FileIcon name={child.name} type={child.type} /></div>
                             <span className={`truncate ${selectedFile?.id === child.id ? 'text-blue-400 font-medium' : 'text-gray-300 group-hover:text-white'}`}>
                               {child.name}
                             </span>
                           </div>
                           <div className="px-4 py-2 text-gray-500 text-xs truncate">
                             {child.date}
                           </div>
                           <div className="px-4 py-2 text-gray-500 text-xs truncate">
                             {child.extension || (child.type === 'folder' ? 'Folder' : 'File')}
                           </div>
                           <div className="px-4 py-2 text-gray-500 text-xs font-mono truncate">
                             {child.size}
                           </div>
                         </div>
                       ))}
                     </div>
                   </div>
                 ) : (
                   /* Grid View */
                   <div className="grid grid-cols-3 lg:grid-cols-4 gap-4 p-4">
                     {sortedFiles.map(child => (
                       <div 
                         key={child.id}
                         onClick={() => child.type === 'folder' ? handleFolderClick(child) : handleFileClick(child)}
                         className={`
                           group cursor-pointer rounded-lg border border-transparent
                           flex flex-col items-center p-4 hover:bg-gray-800 hover:border-gray-700 aspect-square justify-center transition-all
                           ${selectedFile?.id === child.id ? 'bg-blue-500/10 border-blue-500/30' : ''}
                         `}
                       >
                         <div className="mb-3">
                            <FileIcon name={child.name} type={child.type} size="lg" />
                         </div>
                         <div className="flex-1 min-w-0 text-center w-full">
                            <div className={`text-sm font-medium truncate ${selectedFile?.id === child.id ? 'text-blue-400' : 'text-gray-300 group-hover:text-white'}`}>
                              {child.name}
                            </div>
                            <div className="text-xs text-gray-600 mt-1 truncate">
                              {child.size} • {child.date}
                            </div>
                         </div>
                       </div>
                     ))}
                   </div>
                 )}
               </>
             ) : (
               <div className="flex flex-col items-center justify-center h-full text-gray-500">
                  <Folder size={48} className="mb-4 opacity-20" />
                  <p>Folder is empty</p>
               </div>
             )}
          </div>

          {/* Collapsible Terminal Panel */}
          {isTerminalOpen && (
            <div className="h-48 bg-[#0d1117] border-t border-gray-800 flex flex-col shrink-0 animate-in slide-in-from-bottom duration-200">
              <div className="h-8 flex items-center px-4 border-b border-gray-800 justify-between bg-[#0d1117]">
                  <div className="flex items-center space-x-4 text-xs font-mono">
                    <span className="text-blue-400 border-b-2 border-blue-500 py-2 cursor-pointer">Terminal</span>
                    <span className="text-gray-500 hover:text-gray-300 cursor-pointer py-2">Output</span>
                    <span className="text-gray-500 hover:text-gray-300 cursor-pointer py-2">Git</span>
                  </div>
                  <div className="flex items-center space-x-2">
                    <button onClick={() => setTerminalLines([])} className="text-gray-500 hover:text-white" title="Clear">
                      <RefreshCw size={12} />
                    </button>
                    <button onClick={() => setIsTerminalOpen(false)} className="text-gray-500 hover:text-white">
                      <ChevronDown size={14} />
                    </button>
                  </div>
              </div>
              <div className="flex-1 p-3 font-mono text-xs overflow-y-auto custom-scrollbar text-left" ref={terminalRef}>
                  {terminalLines.map((line) => (
                    <div key={line.id} className="mb-1 leading-relaxed">
                      {line.type === 'input' && (
                        <div className="flex text-gray-200">
                          <span className="text-green-500 mr-2">➜</span>
                          <span className="text-blue-400 mr-2">~/Projects/{currentFolder.name}</span>
                          <span>{line.content}</span>
                        </div>
                      )}
                      {line.type === 'info' && <div className="text-gray-400 ml-4">{line.content}</div>}
                      {line.type === 'error' && <div className="text-red-400 ml-4">{line.content}</div>}
                      {line.type === 'success' && <div className="text-green-400 ml-4 whitespace-pre-wrap">{line.content}</div>}
                      {line.type === 'output' && <div className="text-gray-300 ml-4 whitespace-pre-wrap opacity-90">{line.content}</div>}
                    </div>
                  ))}
                  <div className="flex text-gray-200 mt-1">
                    <span className="text-green-500 mr-2">➜</span>
                    <span className="text-blue-400 mr-2">~/Projects/{currentFolder.name}</span>
                    <span className="w-2 h-4 bg-gray-400 animate-pulse block"></span>
                  </div>
              </div>
            </div>
          )}
        </div>

        {/* COLUMN 3: INSPECTOR (Preview Pane) */}
        {selectedFile && (
          <div className="w-80 lg:w-96 bg-[#0d1117] border-l border-gray-800 flex flex-col shrink-0 animate-in slide-in-from-right duration-300">
             
             {/* Inspector Toolbar */}
             <div className="h-12 bg-[#0d1117] border-b border-gray-800 flex items-center justify-between px-4 shrink-0">
                <span className="text-xs font-semibold text-gray-400 uppercase tracking-wider">Preview</span>
                <div className="flex items-center space-x-2">
                  <button className="p-1.5 text-gray-400 hover:text-white hover:bg-gray-800 rounded">
                    <Maximize2 size={14} />
                  </button>
                  <button className="p-1.5 text-gray-400 hover:text-white hover:bg-gray-800 rounded">
                    <X size={14} onClick={() => setSelectedFile(null)} />
                  </button>
                </div>
             </div>

             {/* File Metadata Header */}
             <div className="p-4 border-b border-gray-800 bg-[#161b22]">
                <div className="flex items-start space-x-3 mb-3">
                   <div className="p-2 bg-gray-800 rounded-lg shrink-0">
                      <FileIcon name={selectedFile.name} type={selectedFile.type} size="lg" />
                   </div>
                   <div className="flex-1 overflow-hidden min-w-0">
                      <h3 className="text-sm font-bold text-gray-100 truncate text-left" title={selectedFile.name}>{selectedFile.name}</h3>
                      <p className="text-xs text-gray-500 mt-0.5 text-left">{selectedFile.size || '12 KB'} • {selectedFile.extension || 'Text'}</p>
                   </div>
                </div>
                
                <div className="grid grid-cols-2 gap-2">
                   <button 
                     onClick={handleAiAnalyze}
                     disabled={isAnalyzing}
                     className="flex items-center justify-center gap-2 py-1.5 px-3 bg-blue-600 hover:bg-blue-500 text-white rounded text-xs font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                   >
                     <Sparkles size={12} />
                     {isAnalyzing ? 'Thinking...' : 'Explain Code'}
                   </button>
                   <button className="flex items-center justify-center gap-2 py-1.5 px-3 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded text-xs font-medium border border-gray-700 transition-colors">
                     <Copy size={12} />
                     Copy
                   </button>
                </div>
             </div>

             {/* Content Preview */}
             <div className="flex-1 overflow-auto custom-scrollbar relative bg-[#0d1117] text-left">
               {selectedFile.content ? (
                 <SyntaxHighlight code={selectedFile.content} language={selectedFile.language || 'plaintext'} />
               ) : (
                 <div className="flex flex-col items-center justify-center h-full text-gray-600 space-y-2 p-8 text-center">
                    <FileText size={32} className="opacity-20" />
                    <p className="text-xs">Preview not available</p>
                 </div>
               )}
             </div>

             {/* Bottom Info Bar */}
             <div className="h-8 border-t border-gray-800 bg-[#0d1117] flex items-center justify-between px-3 text-[10px] text-gray-500 shrink-0">
                <span>UTF-8</span>
                <span>{selectedFile.date || 'Unknown'}</span>
                <span>{selectedFile.extension || 'Plain Text'}</span>
             </div>
          </div>
        )}
      </div>
    </div>
  );
};