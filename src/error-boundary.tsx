/**
 * Root-level error boundary. Catches uncaught render-time errors so the
 * entire window doesn't disappear into a white screen. Logs to the
 * console (which `tracing` captures in dev) and shows a minimal
 * recovery UI.
 */

import * as React from "react";

interface State {
  error: Error | null;
}

interface Props {
  children: React.ReactNode;
}

export class ErrorBoundary extends React.Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info);
  }

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-screen w-screen items-center justify-center bg-background p-8">
          <div className="max-w-md space-y-4 text-center">
            <h1 className="text-2xl font-semibold">Something went wrong</h1>
            <p className="text-sm text-muted-foreground">
              The UI hit an unexpected error and recovered before the window vanished.
              Reloading is usually enough; if not, the error below should help.
            </p>
            <pre className="overflow-auto rounded-md bg-muted p-3 text-left text-xs text-muted-foreground">
              {this.state.error.message}
            </pre>
            <button
              type="button"
              onClick={this.handleReload}
              className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:opacity-90"
            >
              Reload window
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
