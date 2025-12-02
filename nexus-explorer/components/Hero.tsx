import React from 'react';
import { Download, ChevronRight } from 'lucide-react';
import { MockApp } from './MockApp';

export const Hero: React.FC = () => {
  return (
    <div className="relative pt-32 pb-20 lg:pt-40 lg:pb-32 overflow-hidden">
      
      <div className="container mx-auto px-4 lg:px-8 relative z-10">
        <div className="flex flex-col items-center gap-12 text-center">
          
          {/* Text Content */}
          <div className="max-w-4xl mx-auto flex flex-col items-center">
            <div className="inline-flex items-center px-3 py-1 rounded-full bg-blue-500/10 border border-blue-500/20 text-blue-400 text-sm font-medium mb-6">
              <span className="flex h-2 w-2 rounded-full bg-blue-400 mr-2 animate-pulse"></span>
              v2.0 is now available
            </div>
            
            <h1 className="text-5xl lg:text-7xl font-bold text-white leading-tight mb-6 tracking-tight">
              The File Explorer <br/>
              <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-purple-500">
                Built for Code.
              </span>
            </h1>
            
            <p className="text-xl text-gray-400 mb-10 leading-relaxed max-w-2xl mx-auto">
              Stop switching windows. Nexus Explorer combines your filesystem, terminal, and git workflow into one beautiful, keyboard-driven interface.
            </p>
            
            <div className="flex flex-col sm:flex-row items-center justify-center gap-4 w-full">
              <button className="w-full sm:w-auto px-8 py-4 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-semibold transition-all flex items-center justify-center shadow-lg shadow-blue-500/25">
                <Download className="mr-2 w-5 h-5" />
                Download for Linux
              </button>
              <button className="w-full sm:w-auto px-8 py-4 bg-gray-800 hover:bg-gray-700 text-gray-300 border border-gray-700 rounded-lg font-semibold transition-all flex items-center justify-center">
                Documentation <ChevronRight className="ml-1 w-4 h-4" />
              </button>
            </div>
            
            <div className="mt-8 text-sm text-gray-500 flex items-center justify-center gap-6">
              <span>Free for personal use</span>
              <span className="w-1 h-1 bg-gray-700 rounded-full"></span>
              <span>macOS & Windows coming soon</span>
            </div>
          </div>

          {/* Mockup Display - Full Width for 3 Columns */}
          <div className="w-full mt-12 relative z-20">
            <div className="relative group">
               {/* Glow effect behind */}
               <div className="absolute -inset-1 bg-gradient-to-r from-blue-600 to-purple-600 rounded-2xl opacity-20 blur-xl group-hover:opacity-30 transition-opacity duration-500"></div>
               
               {/* Main App Container */}
               <div className="relative w-full h-[600px] md:h-[750px] lg:h-[800px] rounded-xl overflow-hidden bg-[#0d1117] shadow-2xl border border-gray-800">
                  <MockApp />
               </div>

               {/* Decorative Badge */}
               <div className="absolute -right-6 -bottom-8 bg-gray-800 border border-gray-700 p-4 rounded-xl shadow-2xl hidden lg:block z-30">
                  <div className="flex items-center gap-3">
                    <div className="bg-green-500/20 p-2 rounded-lg">
                      <div className="w-2 h-2 bg-green-500 rounded-full animate-ping absolute"></div>
                      <div className="w-2 h-2 bg-green-500 rounded-full relative"></div>
                    </div>
                    <div>
                      <div className="text-xs text-gray-400 uppercase font-semibold">System Status</div>
                      <div className="text-white font-mono text-sm">All systems operational</div>
                    </div>
                  </div>
               </div>
            </div>
          </div>

        </div>
      </div>
    </div>
  );
};