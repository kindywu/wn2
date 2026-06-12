import { useState } from 'react'
import { Input, List, Card, Tag, Tabs, Button, message, Space, Typography, Descriptions, Modal } from 'antd'
import { SearchOutlined, CheckOutlined, EditOutlined } from '@ant-design/icons'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import client from '../api/client'
import type { WordOut, DefinitionOut, TranslationOut, ExampleOut, WordRelationOut, WordFormOut } from '../types/api'

const { Text } = Typography

const sourceColor: Record<string, string> = { human: 'gold', stardict: 'blue', wn: 'green', llm: 'orange' }
const posName: Record<string, string> = { n: 'noun', v: 'verb', a: 'adj', r: 'adv', s: 'adj(s)' }

export default function SearchPage() {
  const [query, setQuery] = useState('')
  const [mode, setMode] = useState<'exact' | 'prefix'>('exact')
  const [selectedId, setSelectedId] = useState<number | null>(null)

  const { data: searchData, isLoading, refetch } = useQuery({
    queryKey: ['words', query, mode],
    queryFn: async () => {
      if (!query.trim()) return null
      const resp = await client.get('/words', { params: { q: query, mode, limit: 50 } })
      return resp.data
    },
    enabled: false,
  })

  const words: WordOut[] = searchData?.data || []

  return (
    <div>
      <Space style={{ marginBottom: 24, width: '100%' }} direction="vertical">
        <Input.Search
          size="large"
          placeholder="Search word (e.g. bank, run, happy)..."
          allowClear
          enterButton={<><SearchOutlined /> Search</>}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onSearch={() => { setSelectedId(null); refetch() }}
          addonBefore={
            <select value={mode} onChange={(e) => setMode(e.target.value as 'exact' | 'prefix')}
              style={{ border: 'none', background: 'transparent', cursor: 'pointer', padding: '0 8px' }}>
              <option value="exact">Exact</option>
              <option value="prefix">Prefix</option>
            </select>
          }
        />
      </Space>

      {isLoading && <Card loading />}

      <div style={{ display: 'flex', gap: 24 }}>
        <div style={{ flex: words.length > 0 ? '0 0 350px' : '1' }}>
          <List
            dataSource={words}
            renderItem={(w: WordOut) => (
              <List.Item
                onClick={() => setSelectedId(w.id)}
                style={{
                  cursor: 'pointer',
                  padding: '12px 16px',
                  background: selectedId === w.id ? '#e6f4ff' : undefined,
                  borderRadius: 4,
                }}
              >
                <List.Item.Meta
                  title={
                    <Space>
                      <Text strong style={{ fontSize: 16 }}>{w.word}</Text>
                      {w.pos && <Tag color="purple">{posName[w.pos] || w.pos}</Tag>}
                      {w.curated && <Tag color="gold">curated</Tag>}
                    </Space>
                  }
                  description={
                    <Space size={4}>
                      {w.phonetic && <Text type="secondary">/{w.phonetic}/</Text>}
                      {w.collins > 0 && <Tag color="blue">Collins {w.collins}★</Tag>}
                      {w.oxford && <Tag color="green">Oxford</Tag>}
                      {w.frq && <Tag>frq #{w.frq}</Tag>}
                    </Space>
                  }
                />
              </List.Item>
            )}
            locale={{ emptyText: query ? (isLoading ? 'Searching...' : 'No results') : 'Type a word to search' }}
          />
        </div>

        {selectedId && (
          <div style={{ flex: 1 }}>
            <WordDetail wordId={selectedId} words={words} />
          </div>
        )}
      </div>
    </div>
  )
}

function WordDetail({ wordId, words }: { wordId: number; words: WordOut[] }) {
  const w = words.find((w) => w.id === wordId)
  const queryClient = useQueryClient()

  const { data: defs } = useQuery({
    queryKey: ['definitions', wordId],
    queryFn: async () => {
      const r = await client.get(`/words/${wordId}/definitions`)
      return r.data.data as DefinitionOut[]
    },
  })

  const { data: trans } = useQuery({
    queryKey: ['translations', wordId],
    queryFn: async () => {
      const r = await client.get(`/words/${wordId}/translations`)
      return r.data.data as TranslationOut[]
    },
  })

  const { data: examples } = useQuery({
    queryKey: ['examples', wordId],
    queryFn: async () => {
      const r = await client.get(`/words/${wordId}/examples`)
      return r.data.data as ExampleOut[]
    },
  })

  const { data: relations } = useQuery({
    queryKey: ['relations', wordId],
    queryFn: async () => {
      const r = await client.get(`/words/${wordId}/relations`)
      return r.data.data as WordRelationOut[]
    },
  })

  const { data: forms } = useQuery({
    queryKey: ['forms', wordId],
    queryFn: async () => {
      const r = await client.get(`/words/${wordId}/forms`)
      return r.data.data as WordFormOut[]
    },
  })

  const reviewMut = useMutation({
    mutationFn: async (id: number) => { await client.patch(`/definitions/${id}/review`) },
    onSuccess: () => { message.success('Reviewed'); queryClient.invalidateQueries({ queryKey: ['definitions', wordId] }) },
  })

  const editMut = useMutation({
    mutationFn: async ({ id, text }: { id: number; text: string }) => {
      await client.patch(`/definitions/${id}`, { text })
    },
    onSuccess: () => { message.success('Updated'); queryClient.invalidateQueries({ queryKey: ['definitions', wordId] }) },
  })

  if (!w) return null

  return (
    <Card title={`${w.word}${w.pos ? ` (${posName[w.pos] || w.pos})` : ''}`}
      extra={!w.curated && (
        <Button type="primary" icon={<CheckOutlined />}
          onClick={async () => {
            await client.patch(`/words/${w.id}/curate`)
            message.success('Curated')
            queryClient.invalidateQueries({ queryKey: ['words'] })
          }}>
          Curate
        </Button>
      )}
    >
      <Tabs items={[
        {
          key: 'defs',
          label: `Definitions (${defs?.length || 0})`,
          children: (
            <List dataSource={defs || []} renderItem={(d: DefinitionOut) => (
              <List.Item actions={[
                !d.reviewed && <Button size="small" icon={<CheckOutlined />} onClick={() => reviewMut.mutate(d.id)}>Review</Button>,
                <Button size="small" icon={<EditOutlined />} onClick={() => {
                  Modal.confirm({
                    title: 'Edit Definition',
                    content: <Input.TextArea id="edit-def-text" defaultValue={d.text} rows={3} />,
                    onOk: () => {
                      const input = document.getElementById('edit-def-text') as HTMLTextAreaElement
                      if (input?.value) editMut.mutate({ id: d.id, text: input.value })
                    },
                  })
                }}>Edit</Button>,
              ]}>
                <List.Item.Meta
                  title={<Text>{d.text}</Text>}
                  description={
                    <Space size={4}>
                      <Tag color={sourceColor[d.source]}>{d.source}</Tag>
                      {d.reviewed && <Tag color="green">reviewed</Tag>}
                      {d.modified && <Tag color="gold">modified</Tag>}
                    </Space>
                  }
                />
              </List.Item>
            )} />
          ),
        },
        {
          key: 'trans',
          label: `Translations (${trans?.length || 0})`,
          children: (
            <List dataSource={trans || []} renderItem={(t: TranslationOut) => (
              <List.Item>
                <List.Item.Meta
                  title={t.text}
                  description={<Space size={4}><Tag color={sourceColor[t.source]}>{t.source}</Tag>{t.language !== 'zh' && <Tag>{t.language}</Tag>}</Space>}
                />
              </List.Item>
            )} />
          ),
        },
        {
          key: 'examples',
          label: `Examples (${examples?.length || 0})`,
          children: (
            <List dataSource={examples || []} renderItem={(e: ExampleOut) => (
              <List.Item>
                <List.Item.Meta
                  title={e.text}
                  description={e.translation && <Text type="secondary">{e.translation}</Text>}
                />
              </List.Item>
            )} />
          ),
        },
        {
          key: 'relations',
          label: `Relations (${relations?.length || 0})`,
          children: (
            <List dataSource={relations || []} renderItem={(r: WordRelationOut) => (
              <List.Item>
                <List.Item.Meta
                  title={<Text>{r.related_word}</Text>}
                  description={<Tag>{r.relation_type}</Tag>}
                />
              </List.Item>
            )} />
          ),
        },
        {
          key: 'forms',
          label: `Forms (${forms?.length || 0})`,
          children: (
            <List dataSource={forms || []} renderItem={(f: WordFormOut) => (
              <List.Item>
                <List.Item.Meta
                  title={f.form}
                  description={<Tag>{f.form_type}</Tag>}
                />
              </List.Item>
            )} />
          ),
        },
      ]} />
    </Card>
  )
}
