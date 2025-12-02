import React from 'react';
import { Hero } from './components/Hero';
import { Features } from './components/Features';
import { Command, Github, Twitter } from 'lucide-react';

function App() {
  return (
    <div className="min-h-screen bg-gray-900 text-gray-100 font-sans selection:bg-blue-500/30">
      
      {/* Navigation */}
      <nav className="fixed top-0 w-full z-50 bg-gray-900/80 backdrop-blur-md border-b border-gray-800">
        <div className="container mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <div className="w-8 h-8 bg-gradient-to-br from-blue-500 to-purple-600 rounded-lg flex items-center justify-center text-white font-bold">
              N
            </div>
            <span className="text-xl font-bold tracking-tight">Nexus Explorer</span>
          </div>
          
          <div className="hidden md:flex items-center space-x-8 text-sm font-medium text-gray-400">
            <a href="#" className="hover:text-white transition-colors">Features</a>
            <a href="#" className="hover:text-white transition-colors">Plugins</a>
            <a href="#" className="hover:text-white transition-colors">Changelog</a>
            <a href="#" className="hover:text-white transition-colors">Docs</a>
          </div>

          <div className="flex items-center space-x-4">
             <a href="#" className="text-gray-400 hover:text-white">
               <Github className="w-5 h-5" />
             </a>
             <button className="hidden sm:flex px-4 py-2 bg-white text-gray-900 rounded-md text-sm font-bold hover:bg-gray-100 transition-colors">
               Get Started
             </button>
          </div>
        </div>
      </nav>

      <main>
        <Hero />
        <Features />
      </main>

      {/* Footer */}
      <footer className="border-t border-gray-800 bg-[#0f1117] pt-16 pb-8">
        <div className="container mx-auto px-6">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-8 mb-12">
            <div>
              <h4 className="text-white font-bold mb-4">Product</h4>
              <ul className="space-y-2 text-gray-400 text-sm">
                <li><a href="#" className="hover:text-blue-400">Download</a></li>
                <li><a href="#" className="hover:text-blue-400">Themes</a></li>
                <li><a href="#" className="hover:text-blue-400">Integrations</a></li>
              </ul>
            </div>
            <div>
              <h4 className="text-white font-bold mb-4">Resources</h4>
              <ul className="space-y-2 text-gray-400 text-sm">
                <li><a href="#" className="hover:text-blue-400">Documentation</a></li>
                <li><a href="#" className="hover:text-blue-400">API Reference</a></li>
                <li><a href="#" className="hover:text-blue-400">Community</a></li>
              </ul>
            </div>
            <div>
              <h4 className="text-white font-bold mb-4">Company</h4>
              <ul className="space-y-2 text-gray-400 text-sm">
                <li><a href="#" className="hover:text-blue-400">About</a></li>
                <li><a href="#" className="hover:text-blue-400">Blog</a></li>
                <li><a href="#" className="hover:text-blue-400">Careers</a></li>
              </ul>
            </div>
            <div>
              <h4 className="text-white font-bold mb-4">Legal</h4>
              <ul className="space-y-2 text-gray-400 text-sm">
                <li><a href="#" className="hover:text-blue-400">Privacy</a></li>
                <li><a href="#" className="hover:text-blue-400">Terms</a></li>
              </ul>
            </div>
          </div>
          
          <div className="flex flex-col md:flex-row justify-between items-center pt-8 border-t border-gray-800">
            <div className="text-gray-500 text-sm mb-4 md:mb-0">
              © 2024 Nexus Explorer. All rights reserved.
            </div>
            <div className="flex items-center space-x-6">
               <div className="flex items-center text-gray-500 text-sm">
                 <Command className="w-4 h-4 mr-2" /> 
                 <span>Designed by Developers</span>
               </div>
               <Twitter className="w-5 h-5 text-gray-500 hover:text-white cursor-pointer" />
               <Github className="w-5 h-5 text-gray-500 hover:text-white cursor-pointer" />
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default App;
