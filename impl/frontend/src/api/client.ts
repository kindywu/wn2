import axios from 'axios'
import { message } from 'antd'

const client = axios.create({
  baseURL: '/api/v1',
  timeout: 15_000,
})

client.interceptors.request.use((config) => {
  const token = localStorage.getItem('bearer_token') || 'changeme_in_production'
  config.headers.Authorization = `Bearer ${token}`
  return config
})

client.interceptors.response.use(
  (resp) => resp,
  (error) => {
    const msg = error.response?.data?.error?.message || error.message || 'Request failed'
    message.error(msg)
    return Promise.reject(error)
  },
)

export default client
