import { Layout, Menu } from 'antd'
import { SearchOutlined, ClockCircleOutlined, AuditOutlined, DatabaseOutlined, HistoryOutlined } from '@ant-design/icons'
import { useNavigate, useLocation, Routes, Route } from 'react-router-dom'
import SearchPage from './pages/SearchPage'
import PendingChangesPage from './pages/PendingChangesPage'
import EvaluationsPage from './pages/EvaluationsPage'
import AuditLogPage from './pages/AuditLogPage'
import ImportLogPage from './pages/ImportLogPage'

const { Sider, Content } = Layout

const menuItems = [
  { key: '/', icon: <SearchOutlined />, label: 'Search' },
  { key: '/pending', icon: <ClockCircleOutlined />, label: 'Pending Changes' },
  { key: '/evaluations', icon: <AuditOutlined />, label: 'Evaluations' },
  { key: '/audit-log', icon: <HistoryOutlined />, label: 'Audit Log' },
  { key: '/import-logs', icon: <DatabaseOutlined />, label: 'Import Logs' },
]

function App() {
  const navigate = useNavigate()
  const location = useLocation()

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider breakpoint="lg" collapsedWidth="0">
        <div style={{ color: '#fff', padding: '16px', fontSize: 18, fontWeight: 'bold', textAlign: 'center' }}>
          dict
        </div>
        <Menu
          theme="dark"
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
        />
      </Sider>
      <Layout>
        <Content style={{ margin: 24, padding: 24, background: '#fff', borderRadius: 8, minHeight: 'calc(100vh - 48px)' }}>
          <Routes>
            <Route path="/" element={<SearchPage />} />
            <Route path="/pending" element={<PendingChangesPage />} />
            <Route path="/evaluations" element={<EvaluationsPage />} />
            <Route path="/audit-log" element={<AuditLogPage />} />
            <Route path="/import-logs" element={<ImportLogPage />} />
          </Routes>
        </Content>
      </Layout>
    </Layout>
  )
}

export default App
