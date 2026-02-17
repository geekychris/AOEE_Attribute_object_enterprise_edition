import { BrowserRouter, Routes, Route, NavLink } from 'react-router-dom';
import Dashboard from './components/Dashboard';
import GraphExplorer from './components/GraphExplorer';
import QueryBuilder from './components/QueryBuilder';
import EdgeManager from './components/EdgeManager';
import DatasetLoader from './components/DatasetLoader';
import AdminTools from './components/AdminTools';
import './App.css';

function App() {
  return (
    <BrowserRouter>
      <div className="app">
        <nav className="sidebar">
          <h1>AOEE</h1>
          <ul>
            <li><NavLink to="/">Dashboard</NavLink></li>
            <li><NavLink to="/graph">Graph Explorer</NavLink></li>
            <li><NavLink to="/query">Query Builder</NavLink></li>
            <li><NavLink to="/edges">Edge Manager</NavLink></li>
            <li><NavLink to="/dataset">Dataset Loader</NavLink></li>
            <li><NavLink to="/admin">Admin Tools</NavLink></li>
          </ul>
        </nav>
        <main className="content">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/graph" element={<GraphExplorer />} />
            <Route path="/query" element={<QueryBuilder />} />
            <Route path="/edges" element={<EdgeManager />} />
            <Route path="/dataset" element={<DatasetLoader />} />
            <Route path="/admin" element={<AdminTools />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}

export default App;
