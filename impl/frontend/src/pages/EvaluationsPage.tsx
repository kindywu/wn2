import { List, Card, Button, Space, Tag, message, Typography } from 'antd'
import { CheckOutlined, CloseOutlined } from '@ant-design/icons'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import client from '../api/client'
import type { EvaluationOut } from '../types/api'

const { Text, Paragraph } = Typography

const sevColor: Record<string, string> = { critical: 'red', warning: 'orange', info: 'blue' }

export default function EvaluationsPage() {
  const queryClient = useQueryClient()

  const { data, isLoading } = useQuery({
    queryKey: ['evaluations'],
    queryFn: async () => {
      const r = await client.get('/evaluations', { params: { reviewed: false, limit: 100 } })
      return r.data
    },
  })

  const items: EvaluationOut[] = data?.data || []

  const agreeMut = useMutation({
    mutationFn: async (id: number) => { await client.patch(`/evaluations/${id}/agree`) },
    onSuccess: () => { message.success('Applied suggestion'); queryClient.invalidateQueries({ queryKey: ['evaluations'] }) },
  })

  const dismissMut = useMutation({
    mutationFn: async (id: number) => { await client.patch(`/evaluations/${id}/dismiss`) },
    onSuccess: () => { message.success('Dismissed'); queryClient.invalidateQueries({ queryKey: ['evaluations'] }) },
  })

  return (
    <div>
      <h2>LLM Evaluations</h2>
      <List
        loading={isLoading}
        dataSource={items}
        renderItem={(item) => (
          <List.Item>
            <Card style={{ width: '100%' }} size="small">
              <Space direction="vertical" style={{ width: '100%' }}>
                <Space>
                  <Tag color={sevColor[item.severity]}>{item.severity}</Tag>
                  <Tag>{item.target_table}</Tag>
                  <Tag>{item.dimension}</Tag>
                  <Text strong>Score: {item.score}/5</Text>
                </Space>
                {item.comment && <Paragraph style={{ margin: 0 }}>{item.comment}</Paragraph>}
                {item.suggestion && (
                  <div style={{ padding: 8, background: '#f6ffed', borderRadius: 4 }}>
                    <Text type="secondary">Suggestion: </Text>
                    <Text>{item.suggestion}</Text>
                  </div>
                )}
                <Space>
                  <Button type="primary" size="small" icon={<CheckOutlined />}
                    onClick={() => agreeMut.mutate(item.id)}>
                    Apply Suggestion
                  </Button>
                  <Button size="small" icon={<CloseOutlined />}
                    onClick={() => dismissMut.mutate(item.id)}>
                    Dismiss
                  </Button>
                </Space>
              </Space>
            </Card>
          </List.Item>
        )}
        locale={{ emptyText: 'No evaluations to review' }}
      />
    </div>
  )
}
