import { List, Card, Button, Space, Tag, message, Modal, Input } from 'antd'
import { CheckOutlined, CloseOutlined, EditOutlined } from '@ant-design/icons'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import client from '../api/client'
import type { PendingChangeOut } from '../types/api'

export default function PendingChangesPage() {
  const queryClient = useQueryClient()
  const [editValue, setEditValue] = useState('')
  const [editModalId, setEditModalId] = useState<number | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['pending-changes'],
    queryFn: async () => {
      const r = await client.get('/pending-changes', { params: { status: 'pending', limit: 100 } })
      return r.data
    },
  })

  const items: PendingChangeOut[] = data?.data || []

  const approveMut = useMutation({
    mutationFn: async ({ id, value }: { id: number; value?: string }) => {
      await client.patch(`/pending-changes/${id}/approve`, value ? { value } : {})
    },
    onSuccess: () => { message.success('Approved'); queryClient.invalidateQueries({ queryKey: ['pending-changes'] }) },
  })

  const rejectMut = useMutation({
    mutationFn: async (id: number) => { await client.patch(`/pending-changes/${id}/reject`) },
    onSuccess: () => { message.success('Rejected'); queryClient.invalidateQueries({ queryKey: ['pending-changes'] }) },
  })

  return (
    <div>
      <h2>Pending Changes</h2>
      <List
        loading={isLoading}
        dataSource={items}
        renderItem={(item) => (
          <List.Item>
            <Card style={{ width: '100%' }} size="small">
              <Space direction="vertical" style={{ width: '100%' }}>
                <Space>
                  <Tag color="red">{item.table_name}</Tag>
                  {item.field && <Tag>{item.field}</Tag>}
                  <Tag color="blue">{item.source}</Tag>
                </Space>
                <div style={{ display: 'flex', gap: 24 }}>
                  <div style={{ flex: 1 }}>
                    <div style={{ color: '#999', fontSize: 12 }}>Current value</div>
                    <div style={{ padding: 8, background: '#fff7e6', borderRadius: 4, fontFamily: 'monospace' }}>
                      {item.old_value || <em>NULL</em>}
                    </div>
                  </div>
                  <div style={{ flex: 1 }}>
                    <div style={{ color: '#999', fontSize: 12 }}>Proposed value</div>
                    <div style={{ padding: 8, background: '#f6ffed', borderRadius: 4, fontFamily: 'monospace' }}>
                      {item.new_value || <em>NULL</em>}
                    </div>
                  </div>
                </div>
                <Space>
                  <Button type="primary" size="small" icon={<CheckOutlined />}
                    onClick={() => approveMut.mutate({ id: item.id })}>
                    Approve
                  </Button>
                  <Button size="small" icon={<EditOutlined />}
                    onClick={() => {
                      setEditValue(item.new_value || '')
                      setEditModalId(item.id)
                    }}>
                    Edit & Approve
                  </Button>
                  <Button danger size="small" icon={<CloseOutlined />}
                    onClick={() => rejectMut.mutate(item.id)}>
                    Reject
                  </Button>
                </Space>
              </Space>
            </Card>
          </List.Item>
        )}
        locale={{ emptyText: 'No pending changes — all clear!' }}
      />
      <Modal title="Edit & Approve" open={editModalId !== null}
        onOk={() => {
          if (editModalId) approveMut.mutate({ id: editModalId, value: editValue })
          setEditModalId(null)
        }}
        onCancel={() => setEditModalId(null)}>
        <Input.TextArea value={editValue} onChange={(e) => setEditValue(e.target.value)} rows={4} />
      </Modal>
    </div>
  )
}
