// ecosystem.config.js
module.exports = {
  apps: [
    {
      name: "engine",
      script: "./target/release/aidynamics-trade", // Path to your compiled binary
      env: {
        // Environment variables for all environments
        DATABASE_URL: "your_production_database_url",
        RUST_LOG: "info",
      },
      env_production: {
        // Environment variables specific to the production environment
        NODE_ENV: "production", // If your Rust app uses it.
        API_KEY: "C86PzR7L",
        CLIENT_CODE: "AAAF457795",
        PIN: "5135",
        OTP_TOKEN: "URTSOPBFWOZYBI5XPMGK4R6O5I",
        FIREBASE_URL: "https://ai-dynamics-default-rtdb.firebaseio.com",
        FIREBASE_AUTH_KEY: "AIzaSyAZMe6Tht-SYHE8UK8S0vMJYMzLYOY4lnc",
        WS_URL: "ws://localhost:8003",
        STATUS_URL: "http://localhost:8001/cstatus",
        API_URL: "https://api.aidynamicsplatform.in",
        JWT_TOKEN:
          "eyJhbGciOiJIUzUxMiJ9.eyJ1c2VybmFtZSI6IlM1NTU5MTE3NiIsInJvbGVzIjowLCJ1c2VydHlwZSI6IlVTRVIiLCJ0b2tlbiI6ImV5SmhiR2NpT2lKU1V6STFOaUlzSW5SNWNDSTZJa3BYVkNKOS5leUoxYzJWeVgzUjVjR1VpT2lKamJHbGxiblFpTENKMGIydGxibDkwZVhCbElqb2lkSEpoWkdWZllXTmpaWE56WDNSdmEyVnVJaXdpWjIxZmFXUWlPalFzSW5OdmRYSmpaU0k2SWpNaUxDSmtaWFpwWTJWZmFXUWlPaUpsWW1FeFpUazBOQzFsTkdRM0xUTXdZV1F0WWpZME1DMHdPRGMwTXpRNVpqRTNNRFVpTENKcmFXUWlPaUowY21Ga1pWOXJaWGxmZGpFaUxDSnZiVzVsYldGdVlXZGxjbWxrSWpvMExDSndjbTlrZFdOMGN5STZleUprWlcxaGRDSTZleUp6ZEdGMGRYTWlPaUpoWTNScGRtVWlmWDBzSW1semN5STZJblJ5WVdSbFgyeHZaMmx1WDNObGNuWnBZMlVpTENKemRXSWlPaUpUTlRVMU9URXhOellpTENKbGVIQWlPakUzTWpVMU16RXhPVEFzSW01aVppSTZNVGN5TlRRek5qQXhOaXdpYVdGMElqb3hOekkxTkRNMk1ERTJMQ0pxZEdraU9pSTNZbVJpTURjME1DMWtZVEU0TFRRMk0yTXRZbVJqTUMxaFpUQmhOV1pqWXpobFkyRWlmUS5tcjBCdW92cmFfTnBwU3oxaU04ZC1Yb2QzcHNDSE5GT0JjM0V4TEJmRnhCUm5EOXB2ZXJSak5EVmZoNTFOa2dSTVEzVFhFWlZvRXRvdFBfWmNRdFlSbG9TUFdVUFJpRHV5M29vc1g0U2Q5YnNILUtzVGdzbVZ1UVNqRlRXUkdTTVRDUkZtX3RLbDNSYmhfb0RhZzZFVmhmSFl2VngzZ2psRktWNDB6UnQzaXciLCJBUEktS0VZIjoiU0RyOEdBTmIiLCJpYXQiOjE3MjU0MzYwNzYsImV4cCI6MTcyNTUzMTE5MH0.Kf5Rro76sIpuQTPt5o88SvcOO6Mdeu-s8hNVBCUhFWu2RwMq5ILY9VfV5S6RxdrWWZ9uZxXarhdlcl5VgDXUBw",
        REFRESH_TOKEN:
          "eyJhbGciOiJIUzUxMiJ9.eyJ0b2tlbiI6IlJFRlJFU0gtVE9LRU4iLCJSRUZSRVNILVRPS0VOIjoiZXlKaGJHY2lPaUpTVXpJMU5pSXNJblI1Y0NJNklrcFhWQ0o5LmV5SjFjMlZ5WDNSNWNHVWlPaUpqYkdsbGJuUWlMQ0owYjJ0bGJsOTBlWEJsSWpvaWRISmhaR1ZmY21WbWNtVnphRjkwYjJ0bGJpSXNJbWR0WDJsa0lqb3dMQ0prWlhacFkyVmZhV1FpT2lKbFltRXhaVGswTkMxbE5HUTNMVE13WVdRdFlqWTBNQzB3T0RjME16UTVaakUzTURVaUxDSnJhV1FpT2lKMGNtRmtaVjlyWlhsZmRqRWlMQ0p2Ylc1bGJXRnVZV2RsY21sa0lqb3dMQ0pwYzNNaU9pSnNiMmRwYmw5elpYSjJhV05sSWl3aWMzVmlJam9pVXpVMU5Ua3hNVGMySWl3aVpYaHdJam94TnpJMU5qQTRPRGMyTENKdVltWWlPakUzTWpVME16WXdNVFlzSW1saGRDSTZNVGN5TlRRek5qQXhOaXdpYW5ScElqb2lNV1l6TkdVNVlqVXRZelExTWkwMFlqVTBMV0UzTnpjdFkyRmxOMkZsWXpFNU5tTXpJbjAuRFM5NXZXVUdvWW1xd24tT2tZa1Y4aUQ4VmRhdHFlU1pOWS14alFtZ1RTRjdPRDY2TVI5WUFsWlFlcDdyNUFuV2VUTzRScFhWUnVwblFBelRKMWtTaFFROVdveDFJbkNlbmtnbThqaGlzampHekdpT2hTR0xWOEkxUFlvOXNKNDVvZFZfbFJ6bHl1UmtMbmxmQnJHVGI0WllmMzA3X3l1d0VCSldoaVNrRVNNIiwiaWF0IjoxNzI1NDM2MDc2fQ.lcT-DOml_XfUxiq5tpT9P0e8fnt2PsRG1A7c_KaXT1rNMwF5wa5hOK9yBkts6isQtJsnUE5A-PdYBH0kstI7jg",
        NEW_RUST_PASSWORD: "Pinku@2025",
        NEW_PYTHON_PASSWORD: "Python$5135",
        WEBSERVER_AUTH_TOKEN: "",
        FEED_TOKEN:
          "eyJhbGciOiJIUzUxMiJ9.eyJ1c2VybmFtZSI6IlM1NTU5MTE3NiIsImlhdCI6MTcyNTQzNjA3NiwiZXhwIjoxNzI1NTIyNDc2fQ.kzbXvDnx8megikNM0kzv91qDgG1tZgZ8AGW0iX5EuFH3c3hTkwHUNYeatBs1LxAKU1lYzzLWTtd5qr_HSzIeKA",
        REDIS_HOST: "127.0.0.1",
        REDIS_PASSWORD: "l42OUsgye3YIJ0+sEqXSVdrlGyfCNuGmL8V8lUzY18Q:",
        REDIS_URL: "redis://127.0.0.1/",
      },
    },
  ],
};
