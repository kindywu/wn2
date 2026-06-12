import { Table, Tag } from 'antd'
import { useQuery } from '@tanstack/react-query'
import client from '../api/client'
import type { ChangeLogOut } from '../types/api'

const actionColor: Record<string, string> = { create: 'green', update: 'blue', delete: 'red', review: 'gold', unreview: 'orange' }

export default function AuditLogPage() {
  const { data, isLoading } = useQuery({
    queryKey: ['change-log'],
    queryFn: async () => {
      const r = await client.get('/change-log', { params: { limit: 100 } })
      return r.data
    },
  })

  const items: ChangeLogOut[] = data?.data || []

  return (
    <div>
      <h2>Audit Log</h2>
      <Table
        loading={isLoading}
        dataSource={items}
        rowKey="id"
        columns={[
          { title: 'ID', dataIndex: 'id', width: 80 },
          { title: 'Table', dataIndex: 'table_name', width: 120, render: (v: string) => <Tag>{v}</Tag> },
          { title: 'Row ID', dataIndex: 'row_id', width: 80 },
          { title: 'Field', dataIndex: 'field', width: 100, render: (v: string | null) => v || '—' },
          {
            title: 'Action', dataIndex: 'action', width: 100,
            render: (v: string) => <Tag color={actionColor[v]}>{v}</Tag>,
          },
          { title: 'Old', dataIndex: 'old_value', ellipsis: true, render: (v: string | null) => v || '—' },
          { title: 'New', dataIndex: 'new_value', ellipsis: true, render: (v: string | null) => v || '—' },
          { title: 'Operator', dataIndex: 'operator', width: 100 },
          { title: 'Time', dataIndex: 'changed_at', width: 180, render: (v: string) => new Date(v).toLocaleString() },
        ]}
        pagination={{ pageSize: 50 }}
        size="small"
      />
    </div>
  )
}
