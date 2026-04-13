import { useState } from "react";
import { ChevronDown, ChevronRight, Wrench, CheckCircle, XCircle, Loader } from "lucide-react";

export interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
  result?: string;
  status?: "pending" | "success" | "error";
}

interface ToolCallCardProps {
  toolCall: ToolCall;
  index: number;
}

export function ToolCallCard({ toolCall, index }: ToolCallCardProps) {
  const [expanded, setExpanded] = useState(false);
  
  const statusIcon = {
    pending: <Loader size={14} className="tool-call-spinner" />,
    success: <CheckCircle size={14} className="tool-call-success" />,
    error: <XCircle size={14} className="tool-call-error" />,
  }[toolCall.status || "pending"];

  return (
    <div className="tool-call-card">
      <div className="tool-call-header" onClick={() => setExpanded(!expanded)}>
        <div className="tool-call-title">
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <Wrench size={14} className="tool-call-icon" />
          <span className="tool-call-index">#{index + 1}</span>
          <span className="tool-call-name">{toolCall.name}</span>
        </div>
        <div className="tool-call-status">
          {statusIcon}
        </div>
      </div>
      
      {expanded && (
        <div className="tool-call-body">
          <div className="tool-call-args">
            <div className="tool-call-section-title">Arguments</div>
            <pre className="tool-call-args-content">
              {JSON.stringify(toolCall.arguments, null, 2)}
            </pre>
          </div>
          
          {toolCall.result && (
            <div className="tool-call-result">
              <div className="tool-call-section-title">Result</div>
              <pre className="tool-call-result-content">
                {typeof toolCall.result === "string" 
                  ? toolCall.result 
                  : JSON.stringify(toolCall.result, null, 2)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function parseToolCalls(content: string): ToolCall[] | null {
  try {
    const toolCallMatch = content.match(/<tool_calls>([\s\S]*?)<\/tool_calls>/);
    if (toolCallMatch) {
      const parsed = JSON.parse(toolCallMatch[1]);
      if (Array.isArray(parsed)) {
        return parsed.map((tc: { id?: string; name?: string; input?: Record<string, unknown>; output?: unknown }) => ({
          id: tc.id || `tc-${Math.random().toString(36).slice(2)}`,
          name: tc.name || "unknown",
          arguments: tc.input || {},
          result: tc.output ? (typeof tc.output === "string" ? tc.output : JSON.stringify(tc.output)) : undefined,
          status: tc.output ? "success" : "pending",
        }));
      }
    }
    
    const jsonMatch = content.match(/```json\n([\s\S]*?)\n```/);
    if (jsonMatch) {
      const parsed = JSON.parse(jsonMatch[1]);
      if (parsed.tool_calls && Array.isArray(parsed.tool_calls)) {
        return parsed.tool_calls.map((tc: { id?: string; name?: string; arguments?: Record<string, unknown>; output?: unknown }) => ({
          id: tc.id || `tc-${Math.random().toString(36).slice(2)}`,
          name: tc.name || "unknown",
          arguments: tc.arguments || {},
          result: tc.output ? (typeof tc.output === "string" ? tc.output : JSON.stringify(tc.output)) : undefined,
          status: tc.output ? "success" : "pending",
        }));
      }
    }
  } catch {
    return null;
  }
  return null;
}

interface ToolCallListProps {
  toolCalls: ToolCall[];
}

export function ToolCallList({ toolCalls }: ToolCallListProps) {
  if (!toolCalls || toolCalls.length === 0) return null;
  
  return (
    <div className="tool-call-list">
      {toolCalls.map((tc, i) => (
        <ToolCallCard key={tc.id} toolCall={tc} index={i} />
      ))}
    </div>
  );
}
