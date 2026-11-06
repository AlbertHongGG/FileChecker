import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import TitleBar from "./components/layout/TitleBar";
import { HomeView } from "./features/home";
import { CompareWorkspace } from "./features/compare";
import { NotificationContainer } from "./components/common/Notification";
import "./App.css";

export type ViewState = 'home' | 'compare';

function App() {
  const [viewState, setViewState] = useState<ViewState>('home');

  return (
    <div className="app-container">
      <TitleBar />
      <NotificationContainer />
      
      <main className="main-content">
        <AnimatePresence mode="wait">
          {viewState === 'home' && (
            <motion.div
              key="home"
              initial={{ opacity: 0, scale: 0.98 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 1.02 }}
              transition={{ duration: 0.3 }}
              style={{ width: '100%', height: '100%' }}
            >
              <HomeView onEnter={() => setViewState('compare')} />
            </motion.div>
          )}

          {viewState === 'compare' && (
            <motion.div
              key="compare"
              initial={{ opacity: 0, y: 15 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -15 }}
              transition={{ duration: 0.3 }}
              style={{ width: '100%', height: '100%' }}
            >
              <CompareWorkspace onBack={() => setViewState('home')} />
            </motion.div>
          )}
        </AnimatePresence>
      </main>
    </div>
  );
}

export default App;
