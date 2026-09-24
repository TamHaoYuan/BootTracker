import { Button, Result } from 'antd';
import { useNavigate } from 'react-router-dom';
import { HomeOutlined } from '@ant-design/icons';

function NotFound() {
  const navigate = useNavigate();
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '70vh',
      }}
    >
      <Result
        status="404"
        title="404"
        subTitle="抱歉，您访问的页面不存在。"
        extra={
          <Button
            type="primary"
            icon={<HomeOutlined />}
            onClick={() => navigate('/dashboard')}
          >
            返回首页
          </Button>
        }
      />
    </div>
  );
}

export default NotFound;
