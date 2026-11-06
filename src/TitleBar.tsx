import { Window } from "@tauri-apps/api/window";
import { Minus, Square, X, Sun, Moon } from "lucide-react";
import { useTheme } from "./contexts/ThemeContext";
import { Tooltip } from "./components/Tooltip";

export default function TitleBar() {
  const { theme, toggleTheme } = useTheme();

  const handleMinimize = () => {
    Window.getCurrent().minimize();
  };

  const handleToggleMaximize = async () => {
    const current = Window.getCurrent();
    const isMaximized = await current.isMaximized();
    if (isMaximized) {
      current.unmaximize();
    } else {
      current.maximize();
    }
  };

  const handleClose = () => {
    Window.getCurrent().close();
  };

  return (
    <div data-tauri-drag-region className="titlebar">
      <div data-tauri-drag-region className="titlebar-drag-region">
        <span className="titlebar-title" data-tauri-drag-region>FileChecker</span>
      </div>
      <div className="titlebar-controls">
        <Tooltip content={theme === 'dark' ? "Light" : "Dark"} position="bottom">
          <button className="icon-btn" onClick={toggleTheme} style={{ marginRight: '4px' }}>
            {theme === 'dark' ? <Sun size={14} /> : <Moon size={14} />}
          </button>
        </Tooltip>
        
        <button className="icon-btn" onClick={handleMinimize} title="Minimize">
          <Minus size={15} />
        </button>
        <button className="icon-btn" onClick={handleToggleMaximize} title="Maximize">
          <Square size={13} />
        </button>
        <button className="icon-btn danger" onClick={handleClose} title="Close">
          <X size={15} />
        </button>
      </div>
    </div>
  );
}
