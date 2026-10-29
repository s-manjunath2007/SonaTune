import React, { useState, useEffect, useRef } from 'react';

interface MenuBarProps {
  onNavigate: (page: string) => void;
  onResetDsp: () => void;
  onShowAbout: () => void;
  isEnhancementActive: boolean;
  onToggleEnhancement: () => void;
}

export const MenuBar: React.FC<MenuBarProps> = ({
  onNavigate,
  onResetDsp,
  onShowAbout,
  isEnhancementActive,
  onToggleEnhancement,
}) => {
  const [activeMenu, setActiveMenu] = useState<string | null>(null);
  const barRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (barRef.current && !barRef.current.contains(e.target as Node)) {
        setActiveMenu(null);
      }
    };
    window.addEventListener('mousedown', handleClickOutside);
    return () => window.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleMenuClick = (menu: string) => {
    setActiveMenu(activeMenu === menu ? null : menu);
  };

  const handleAction = (action: () => void) => {
    setActiveMenu(null);
    action();
  };

  return (
    <div className="menu-bar" ref={barRef}>
      {/* File Menu */}
      <div className={`menu-item ${activeMenu === 'file' ? 'active' : ''}`} onClick={() => handleMenuClick('file')}>
        File
        {activeMenu === 'file' && (
          <div className="menu-dropdown">
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('audio'))}>
              <span>Open Audio File...</span>
              <span className="dropdown-shortcut">Ctrl+O</span>
            </div>
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('video'))}>
              <span>Open Video File...</span>
              <span className="dropdown-shortcut">Ctrl+Shift+O</span>
            </div>
            <div className="dropdown-divider" />
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('settings'))}>
              <span>Settings...</span>
              <span className="dropdown-shortcut">Ctrl+,</span>
            </div>
            <div className="dropdown-divider" />
            <div className="dropdown-item" onClick={() => handleAction(() => window.close())}>
              <span>Exit</span>
              <span className="dropdown-shortcut">Alt+F4</span>
            </div>
          </div>
        )}
      </div>

      {/* Edit Menu */}
      <div className={`menu-item ${activeMenu === 'edit' ? 'active' : ''}`} onClick={() => handleMenuClick('edit')}>
        Edit
        {activeMenu === 'edit' && (
          <div className="menu-dropdown">
            <div className="dropdown-item" onClick={() => handleAction(onResetDsp)}>
              <span>Reset Sound Controls to Defaults</span>
            </div>
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('presets'))}>
              <span>Manage Presets...</span>
            </div>
          </div>
        )}
      </div>

      {/* View Menu */}
      <div className={`menu-item ${activeMenu === 'view' ? 'active' : ''}`} onClick={() => handleMenuClick('view')}>
        View
        {activeMenu === 'view' && (
          <div className="menu-dropdown">
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('sound'))}>
              <span>Sound (Main Enhancement)</span>
              <span className="dropdown-shortcut">Ctrl+1</span>
            </div>
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('audio'))}>
              <span>Audio Files Studio</span>
              <span className="dropdown-shortcut">Ctrl+2</span>
            </div>
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('video'))}>
              <span>Video Audio</span>
              <span className="dropdown-shortcut">Ctrl+3</span>
            </div>
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('devices'))}>
              <span>Devices</span>
              <span className="dropdown-shortcut">Ctrl+4</span>
            </div>
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('presets'))}>
              <span>Presets Library</span>
              <span className="dropdown-shortcut">Ctrl+5</span>
            </div>
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('settings'))}>
              <span>Preferences</span>
              <span className="dropdown-shortcut">Ctrl+6</span>
            </div>
          </div>
        )}
      </div>

      {/* Audio Menu */}
      <div className={`menu-item ${activeMenu === 'audio' ? 'active' : ''}`} onClick={() => handleMenuClick('audio')}>
        Audio
        {activeMenu === 'audio' && (
          <div className="menu-dropdown">
            <div className="dropdown-item" onClick={() => handleAction(onToggleEnhancement)}>
              <span>{isEnhancementActive ? 'Bypass DSP Enhancement' : 'Enable DSP Enhancement'}</span>
              <span className="dropdown-shortcut">Space</span>
            </div>
            <div className="dropdown-item" onClick={() => handleAction(() => onNavigate('devices'))}>
              <span>Audio Output Devices...</span>
            </div>
          </div>
        )}
      </div>

      {/* Help Menu */}
      <div className={`menu-item ${activeMenu === 'help' ? 'active' : ''}`} onClick={() => handleMenuClick('help')}>
        Help
        {activeMenu === 'help' && (
          <div className="menu-dropdown">
            <div className="dropdown-item" onClick={() => handleAction(onShowAbout)}>
              <span>About SonaTune...</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
