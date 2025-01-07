import { check } from 'k6';
import http from 'k6/http';

export const options = {
  thresholds: {
    http_req_failed: ['rate<0.01'], // http errors should be less than 1%
    http_req_duration: ['p(99)<1000'], // 99% of requests should be below 1s
  },
  scenarios: {
    breaking: {
      executor: 'ramping-vus',
      stages: [
        { duration: '10s', target: 20 },
        { duration: '10s', target: 20 },
        { duration: '10s', target: 40 },
        { duration: '10s', target: 60 },
        { duration: '10s', target: 80 },
        { duration: '10s', target: 100 },
        { duration: '10s', target: 120 },
        { duration: '10s', target: 140 },
      ],
    }
  },
};

export default function () {
  const res = http.get('http://127.0.0.1:3000');
  check(res, {
    'response code was 200': (res) => res.status == 200,
  });
}
