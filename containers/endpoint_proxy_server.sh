#!/usr/bin/env sh
BIN_LOCATION=/usr/local/bin/endpoint_proxy

if [ "$ENABLE_COOKIES" = true ];then
  FLAG_ENABLE_COOKIES=1;
fi

exec "$BIN_LOCATION" \
${ROUTE_CONF_LOCATION:+"--config-file=$ROUTE_CONF_LOCATION"} \
${HTTP_BIND:+"--bind=$HTTP_BIND"} \
${HTTP_PORT:+"--port=$HTTP_PORT"} \
${FLAG_ENABLE_COOKIES:+"--enable-cookies"} \
${HTTP_WORKER_COUNT:+"--worker-count=$HTTP_WORKER_COUNT"} \
${HTTP_PROXY_URL:+"--proxy-url=$HTTP_PROXY_URL"} \
${HTTP_PROXY_USER:+"--proxy-auth-user=$HTTP_PROXY_USER"} \
${HTTP_PROXY_PASS:+"--proxy-auth-pass=$HTTP_PROXY_PASS"} \
${LOG_LEVEL:+"--log-level=$LOG_LEVEL"}