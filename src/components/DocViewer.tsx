import React, { useState } from 'react';
import { ProtocolDoc } from '../types/protocol';
import { Copy, Check, FileDown, BookOpen } from 'lucide-react';

interface DocViewerProps {
  doc: ProtocolDoc;
}

export const DocViewer: React.FC<DocViewerProps> = ({ doc }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(doc.markdownContent);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleDownload = () => {
    const blob = new Blob([doc.markdownContent], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${doc.rfcNumber.toLowerCase()}_${doc.id}.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  // Simple clean markdown parser for headings, lists, tables, code blocks and paragraphs
  const renderFormattedContent = (content: string) => {
    const lines = content.split('\n');
    const elements: React.ReactNode[] = [];
    let currentCodeBlock: string[] | null = null;
    let tableRows: string[][] = [];
    let isTable = false;

    const flushTable = (key: number) => {
      if (tableRows.length > 0) {
        const header = tableRows[0];
        const body = tableRows.slice(1);
        elements.push(
          <div key={`table-${key}`} className="my-6 overflow-x-auto rounded-lg border border-neutral-800 bg-neutral-900/60">
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-neutral-800 bg-neutral-900/90 text-neutral-300 font-semibold">
                  {header.map((col, idx) => (
                    <th key={idx} className="py-3 px-4 tracking-wide font-mono">
                      {col.trim()}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-neutral-850">
                {body.map((row, rIdx) => (
                  <tr key={rIdx} className="hover:bg-neutral-800/40 transition-colors">
                    {row.map((cell, cIdx) => (
                      <td key={cIdx} className="py-2.5 px-4 text-neutral-300 font-sans">
                        {cell.trim()}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
        tableRows = [];
        isTable = false;
      }
    };

    lines.forEach((line, index) => {
      // Code blocks
      if (line.startsWith('```')) {
        if (currentCodeBlock === null) {
          if (isTable) flushTable(index);
          currentCodeBlock = [];
        } else {
          elements.push(
            <pre key={`code-${index}`} className="my-4 p-4 rounded-lg bg-neutral-950 border border-neutral-800 text-amber-300/90 font-mono text-xs overflow-x-auto leading-relaxed">
              <code>{currentCodeBlock.join('\n')}</code>
            </pre>
          );
          currentCodeBlock = null;
        }
        return;
      }

      if (currentCodeBlock !== null) {
        currentCodeBlock.push(line);
        return;
      }

      // Markdown Tables
      if (line.trim().startsWith('|') && line.trim().endsWith('|')) {
        isTable = true;
        // Ignore separator row like |---|---|
        if (line.includes('---')) return;
        const cells = line.split('|').slice(1, -1);
        tableRows.push(cells);
        return;
      } else if (isTable) {
        flushTable(index);
      }

      // Headings
      if (line.startsWith('# ')) {
        elements.push(
          <h1 key={index} className="text-2xl font-bold text-neutral-100 tracking-tight mt-6 mb-3">
            {line.replace('# ', '')}
          </h1>
        );
      } else if (line.startsWith('## ')) {
        elements.push(
          <h2 key={index} className="text-xl font-bold text-neutral-100 tracking-tight mt-6 mb-2 border-b border-neutral-800/70 pb-2">
            {line.replace('## ', '')}
          </h2>
        );
      } else if (line.startsWith('### ')) {
        elements.push(
          <h3 key={index} className="text-base font-semibold text-amber-400 mt-5 mb-2">
            {line.replace('### ', '')}
          </h3>
        );
      } else if (line.startsWith('- ')) {
        elements.push(
          <li key={index} className="ml-5 list-disc text-sm text-neutral-300 my-1 leading-relaxed">
            {line.replace('- ', '')}
          </li>
        );
      } else if (line.trim() === '---') {
        elements.push(<hr key={index} className="my-6 border-neutral-800" />);
      } else if (line.trim().length > 0) {
        elements.push(
          <p key={index} className="text-sm text-neutral-300 leading-relaxed my-2">
            {line}
          </p>
        );
      }
    });

    if (isTable) flushTable(lines.length);

    return elements;
  };

  return (
    <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 lg:p-8">
      {/* Document Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-6 border-b border-neutral-800">
        <div>
          <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 mb-1">
            <span className="text-amber-400 font-semibold">{doc.rfcNumber}</span>
            <span aria-hidden="true">·</span>
            <span>{doc.badge}</span>
            <span aria-hidden="true">·</span>
            <span>STANDARD SPECIFICATION</span>
          </div>
          <h1 className="text-xl lg:text-2xl font-bold text-neutral-100 tracking-tight">
            {doc.title}
          </h1>
          <p className="text-sm text-neutral-400 mt-1 max-w-3xl leading-relaxed">
            {doc.summary}
          </p>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={handleCopy}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-neutral-300 bg-neutral-800 hover:bg-neutral-750 border border-neutral-700/60 rounded-lg transition-colors"
            title="Copy Markdown Source"
          >
            {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
            <span>{copied ? 'Copied' : 'Copy RFC'}</span>
          </button>
          <button
            onClick={handleDownload}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-neutral-300 bg-neutral-800 hover:bg-neutral-750 border border-neutral-700/60 rounded-lg transition-colors"
            title="Download Markdown Document"
          >
            <FileDown className="w-3.5 h-3.5" />
            <span>Download</span>
          </button>
        </div>
      </div>

      {/* Rendered Document Body */}
      <div className="prose prose-invert max-w-none pt-4">
        {renderFormattedContent(doc.markdownContent)}
      </div>

      {/* Footer Citation */}
      <div className="mt-8 pt-4 border-t border-neutral-800 flex items-center justify-between text-xs text-neutral-500 font-mono">
        <div className="flex items-center gap-2">
          <BookOpen className="w-3.5 h-3.5 text-amber-500" />
          <span>Sardhan Core Protocol Standard · Zero-Premine Layer-1</span>
        </div>
        <span>Status: RFC Proposed & Pending Review</span>
      </div>
    </div>
  );
};
