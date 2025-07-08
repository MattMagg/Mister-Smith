import React, { useState, useRef, useCallback } from 'react';
import Editor from '@monaco-editor/react';
import { Terminal, Play, Trash2, Download } from 'lucide-react';

interface DebugCommand {
  id: string;
  command: string;
  timestamp: Date;
  output?: string;
  error?: string;
  status: 'pending' | 'success' | 'error';
}

interface DebugConsoleProps {
  onExecuteCommand?: (command: string) => Promise<string>;
  className?: string;
}

export const DebugConsole: React.FC<DebugConsoleProps> = ({
  onExecuteCommand,
  className,
}) => {
  const [commands, setCommands] = useState<DebugCommand[]>([]);
  const [currentCommand, setCurrentCommand] = useState('');
  const [isExecuting, setIsExecuting] = useState(false);
  const editorRef = useRef<any>(null);
  
  const handleEditorMount = (editor: any) => {
    editorRef.current = editor;
    editor.focus();
  };
  
  const executeCommand = useCallback(async () => {
    if (!currentCommand.trim() || isExecuting) return;
    
    const newCommand: DebugCommand = {
      id: `cmd-${Date.now()}`,
      command: currentCommand,
      timestamp: new Date(),
      status: 'pending',
    };
    
    setCommands(prev => [...prev, newCommand]);
    setIsExecuting(true);
    
    try {
      if (onExecuteCommand) {
        const output = await onExecuteCommand(currentCommand);
        setCommands(prev =>
          prev.map(cmd =>
            cmd.id === newCommand.id
              ? { ...cmd, output, status: 'success' }
              : cmd
          )
        );
      } else {
        // Mock execution for demo
        await new Promise(resolve => setTimeout(resolve, 1000));
        setCommands(prev =>
          prev.map(cmd =>
            cmd.id === newCommand.id
              ? { ...cmd, output: `Executed: ${currentCommand}`, status: 'success' }
              : cmd
          )
        );
      }
    } catch (error) {
      setCommands(prev =>
        prev.map(cmd =>
          cmd.id === newCommand.id
            ? { ...cmd, error: error.message, status: 'error' }
            : cmd
        )
      );
    } finally {
      setIsExecuting(false);
      setCurrentCommand('');
    }
  }, [currentCommand, isExecuting, onExecuteCommand]);
  
  const clearConsole = () => {
    setCommands([]);
  };
  
  const exportLogs = () => {
    const logs = commands
      .map(cmd => {
        const timestamp = cmd.timestamp.toISOString();
        const output = cmd.output || cmd.error || '';
        return `[${timestamp}] ${cmd.command}\n${output}\n`;
      })
      .join('\n');
    
    const blob = new Blob([logs], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `debug-log-${Date.now()}.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };
  
  return (
    <div className={`flex flex-col h-full bg-gray-900 rounded-lg border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-700">
        <div className="flex items-center gap-2">
          <Terminal className="w-5 h-5 text-blue-500" />
          <h3 className="text-lg font-medium text-gray-200">Debug Console</h3>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={exportLogs}
            className="p-2 text-gray-400 hover:text-gray-200 transition-colors"
            title="Export logs"
          >
            <Download className="w-4 h-4" />
          </button>
          <button
            onClick={clearConsole}
            className="p-2 text-gray-400 hover:text-gray-200 transition-colors"
            title="Clear console"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      </div>
      
      {/* Command History */}
      <div className="flex-1 overflow-y-auto p-4 font-mono text-sm">
        {commands.length === 0 ? (
          <div className="text-gray-500">
            Type debug commands below. Try:
            <div className="mt-2 space-y-1">
              <div>• debug.trace &lt;discovery-id&gt;</div>
              <div>• debug.agent &lt;agent-id&gt;</div>
              <div>• debug.metrics.snapshot</div>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            {commands.map(cmd => (
              <div key={cmd.id} className="space-y-1">
                <div className="flex items-start gap-2">
                  <span className="text-gray-500">
                    {cmd.timestamp.toLocaleTimeString()}
                  </span>
                  <span className="text-blue-400 flex-1">$ {cmd.command}</span>
                </div>
                {cmd.status === 'pending' && (
                  <div className="text-gray-400 pl-20">Executing...</div>
                )}
                {cmd.output && (
                  <pre className="text-gray-300 pl-20 whitespace-pre-wrap">
                    {cmd.output}
                  </pre>
                )}
                {cmd.error && (
                  <pre className="text-red-400 pl-20 whitespace-pre-wrap">
                    Error: {cmd.error}
                  </pre>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
      
      {/* Command Input */}
      <div className="border-t border-gray-700 p-4">
        <div className="flex items-center gap-2">
          <div className="flex-1 monaco-editor-container h-20">
            <Editor
              height="80px"
              defaultLanguage="shell"
              theme="vs-dark"
              value={currentCommand}
              onChange={(value) => setCurrentCommand(value || '')}
              onMount={handleEditorMount}
              options={{
                minimap: { enabled: false },
                lineNumbers: 'off',
                glyphMargin: false,
                folding: false,
                lineDecorationsWidth: 0,
                lineNumbersMinChars: 0,
                overviewRulerLanes: 0,
                scrollbar: { vertical: 'hidden', horizontal: 'hidden' },
                wordWrap: 'on',
                fontSize: 12,
                fontFamily: 'monospace',
              }}
            />
          </div>
          <button
            onClick={executeCommand}
            disabled={isExecuting || !currentCommand.trim()}
            className="p-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            title="Execute command (Enter)"
          >
            <Play className="w-5 h-5" />
          </button>
        </div>
      </div>
    </div>
  );
};