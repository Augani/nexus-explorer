import React from 'react';
import { Terminal, Cpu, GitBranch, Zap, Layers, Globe } from 'lucide-react';

const features = [
  {
    icon: <Terminal className="w-6 h-6 text-blue-400" />,
    title: "Integrated Terminal",
    description: "Never leave your file manager. Execute git commands, run scripts, and manage dependencies directly from the bottom panel."
  },
  {
    icon: <GitBranch className="w-6 h-6 text-green-400" />,
    title: "First-class Git Support",
    description: "See file status, modified lines, and branch information at a glance. Visual diffs integrated right into the preview pane."
  },
  {
    icon: <Cpu className="w-6 h-6 text-purple-400" />,
    title: "AI Powered Context",
    description: "Built-in Gemini integration understands your project structure. Ask questions about your code without opening an editor."
  },
  {
    icon: <Zap className="w-6 h-6 text-yellow-400" />,
    title: "Blazing Fast",
    description: "Written in Rust with a React frontend. Nexus Explorer handles millions of files with zero lag and instant search."
  },
  {
    icon: <Layers className="w-6 h-6 text-orange-400" />,
    title: "Workspace Layouts",
    description: "Save your window configuration for different projects. Switch between 'Coding', 'Writing', and 'Review' modes instantly."
  },
  {
    icon: <Globe className="w-6 h-6 text-cyan-400" />,
    title: "Remote File Systems",
    description: "Connect to S3, FTP, SSH, and Google Drive as if they were local folders. Drag and drop works seamlessly."
  }
];

export const Features: React.FC = () => {
  return (
    <section className="py-24 bg-gray-900 relative overflow-hidden">
      {/* Background decoration */}
      <div className="absolute top-0 left-0 w-full h-px bg-gradient-to-r from-transparent via-gray-700 to-transparent opacity-50"></div>
      
      <div className="container mx-auto px-6">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <h2 className="text-3xl font-bold text-white mb-4">Everything you need, right where you need it.</h2>
          <p className="text-gray-400 text-lg">Nexus Explorer replaces Finder, Explorer, and your Terminal with a single, unified workspace designed for modern development.</p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
          {features.map((feature, idx) => (
            <div key={idx} className="bg-gray-800/50 backdrop-blur-sm border border-gray-700 rounded-xl p-6 hover:bg-gray-800 transition-all duration-300 hover:-translate-y-1 hover:border-gray-600 group">
              <div className="w-12 h-12 bg-gray-900 rounded-lg flex items-center justify-center mb-4 group-hover:scale-110 transition-transform duration-300 border border-gray-700">
                {feature.icon}
              </div>
              <h3 className="text-xl font-semibold text-white mb-2">{feature.title}</h3>
              <p className="text-gray-400 leading-relaxed">{feature.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};
