import { Table, Tag } from 'antd'
import { useQuery } from '@tanstack/react-query'
import client from '../api/client'
import type { ImportLogOut } from '../types/api'

const statusColor: Record<string, string> = { running: 'blue', completed: 'green', failed: 'red' }

export default function ImportLogPage() {
  const { data, isLoading } = useQuery({
    queryKey: ['import-logs'],
    queryFn: async () => {
      const r = await client.get('/import-logs')
      return r.data
    },
  })

  const items: ImportLogOut[] = data?.data || []

  return (
    <div>
      <h2>Import Logs</h2>
      <Table
        loading={isLoading}
        dataSource={items}
        rowKey="id"
        columns={[
          { title: 'ID', dataIndex: 'id', width: 60 },
          {
            title: 'Source', dataIndex: 'source', width: 90,
            render: (v: string) => <Tag color={v === 'stardict' ? 'blue' : 'green'}>{v}</Tag>,
          },
          { title: 'Version', dataIndex: 'source_version', width: 120, render: (v: string | null) => v || '—' },
          { title: 'Mode', dataIndex: 'mode', width: 100 },
          {
            title: 'Status', dataIndex: 'status', width: 100,
            render: (v: string) => <Tag color={statusColor[v]}>{v}</Tag>,
          },
          { title: 'New Words', dataIndex: 'new_words', width: 100 },
          { title: 'Updated', dataIndex: 'updated_words', width: 90 },
          { title: 'Conflicts', dataIndex: 'conflicts', width: 90 },
          { title: 'Started', dataIndex: 'started_at', width: 180, render: (v: string) => new Date(v).toLocaleString() },
          { title: 'Finished', dataIndex: 'finished_at', width: 180, render: (v: string | null) => v ? new Date(v).toLocaleString() : '—' },
        ]}
        pagination={{ pageSize: 20 }}
        size="small"
      />
    </div>
  )
}
