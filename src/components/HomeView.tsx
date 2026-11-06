import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { FolderSync } from 'lucide-react';
import { Tooltip } from './Tooltip';

interface HomeViewProps {
  onEnter: () => void;
}

export default function HomeView({ onEnter }: HomeViewProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [isClicked, setIsClicked] = useState(false);

  const handleClick = () => {
    setIsClicked(true);
    setTimeout(() => {
      onEnter();
      setIsClicked(false);
    }, 450);
  };

  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100%',
      width: '100%',
      position: 'relative',
      overflow: 'hidden',
      background: 'var(--bg-base)'
    }}>
      {/* Interactive Aurora Portal */}
      <div 
        style={{ position: 'relative', display: 'flex', alignItems: 'center', justifyContent: 'center', width: 400, height: 400 }}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      >
        {/* Outer Nebula Glow */}
        <motion.div
          animate={{
            rotate: 360,
            scale: isHovered ? 1.15 : 1,
            opacity: isHovered ? 0.7 : 0.25,
          }}
          transition={{
            rotate: { duration: 20, repeat: Infinity, ease: "linear" },
            scale: { duration: 0.8, ease: "easeOut" },
            opacity: { duration: 0.8, ease: "easeOut" }
          }}
          style={{
            position: 'absolute',
            width: '280px',
            height: '280px',
            borderRadius: '50%',
            background: 'conic-gradient(from 0deg, #0ea5e9, #6366f1, #a855f7, #ec4899, #0ea5e9)',
            filter: 'blur(50px)',
            pointerEvents: 'none'
          }}
        />

        {/* Counter-Rotating Nebula */}
        <motion.div
          animate={{
            rotate: -360,
            scale: isHovered ? 1.05 : 0.85,
            opacity: isHovered ? 0.5 : 0.15,
          }}
          transition={{
            rotate: { duration: 25, repeat: Infinity, ease: "linear" },
            scale: { duration: 1, ease: "easeOut" },
            opacity: { duration: 1, ease: "easeOut" }
          }}
          style={{
            position: 'absolute',
            width: '220px',
            height: '220px',
            borderRadius: '50%',
            background: 'conic-gradient(from 180deg, #ec4899, #a855f7, #6366f1, #0ea5e9, #ec4899)',
            filter: 'blur(40px)',
            pointerEvents: 'none'
          }}
        />

        {/* Orbit Ring */}
        <motion.div
          animate={{ rotate: 360 }}
          transition={{ duration: 30, repeat: Infinity, ease: "linear" }}
          style={{
            position: 'absolute',
            width: '220px',
            height: '220px',
            borderRadius: '50%',
            border: '1px dashed var(--border-subtle)',
            pointerEvents: 'none',
            opacity: isHovered ? 0.6 : 0.15,
            transition: 'opacity 0.6s ease'
          }}
        >
          <div style={{ 
            position: 'absolute', top: -4, left: '50%', transform: 'translateX(-50%)',
            width: 8, height: 8, borderRadius: '50%', background: '#0ea5e9',
            boxShadow: '0 0 12px #0ea5e9'
          }} />
        </motion.div>

        {/* Core Glass Button */}
        <Tooltip content="Start Comparison" position="bottom">
          <motion.button
            onClick={handleClick}
            animate={{
              scale: isClicked ? 0.85 : isHovered ? 1.06 : 1,
              boxShadow: isHovered 
                ? '0 20px 40px rgba(0, 0, 0, 0.12), inset 0 0 0 1px rgba(255, 255, 255, 0.25), inset 0 0 20px rgba(255, 255, 255, 0.15)' 
                : '0 10px 30px rgba(0, 0, 0, 0.05), inset 0 0 0 1px rgba(150, 150, 150, 0.12), inset 0 0 0px rgba(255, 255, 255, 0)',
            }}
            transition={{ type: 'spring', stiffness: 400, damping: 25 }}
            style={{
              position: 'relative',
              width: '120px',
              height: '120px',
              borderRadius: '50%',
              background: 'rgba(150, 150, 150, 0.08)',
              backdropFilter: 'blur(24px)',
              WebkitBackdropFilter: 'blur(24px)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
              border: 'none',
              outline: 'none',
              zIndex: 10
            }}
          >
            <motion.div
              animate={{
                rotate: isClicked ? 180 : 0,
                scale: isClicked ? 0 : 1,
                opacity: isClicked ? 0 : 1
              }}
              transition={{ duration: 0.4, ease: "backIn" }}
            >
              <FolderSync 
                size={44} 
                strokeWidth={1.5}
                style={{ 
                  color: isHovered ? 'var(--text-primary)' : 'var(--text-secondary)',
                  transition: 'color 0.3s ease',
                  filter: isHovered ? 'drop-shadow(0 0 8px rgba(255,255,255,0.3))' : 'none'
                }} 
              />
            </motion.div>
            
            <AnimatePresence>
              {isClicked && (
                <motion.div
                  initial={{ scale: 0.5, opacity: 0.8 }}
                  animate={{ scale: 2.5, opacity: 0 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.5, ease: "easeOut" }}
                  style={{
                    position: 'absolute',
                    width: '100%',
                    height: '100%',
                    borderRadius: '50%',
                    background: 'rgba(255, 255, 255, 0.25)',
                    pointerEvents: 'none'
                  }}
                />
              )}
            </AnimatePresence>
          </motion.button>
        </Tooltip>
      </div>
    </div>
  );
}
