#!/bin/bash 

echo_blue_bold() {
  echo -e "\033[1;34m$1\033[0m"
}

echo_blue_bold " hurl/chat_completion.hurl >"
hurl --variable USER="$USER" --variable OPENAI_API_KEY="$OPENAI_API_KEY" hurl/chat_completion.hurl
echo ""
echo ""
echo_blue_bold " hurl/reservoir-view.hurl >"
hurl --variable USER="$USER" --variable OPENAI_API_KEY="$OPENAI_API_KEY" hurl/reservoir-view.hurl
echo ""
echo ""
echo_blue_bold " hurl/reservoir-search.hurl >"
hurl --variable USER="$USER" --variable OPENAI_API_KEY="$OPENAI_API_KEY" hurl/reservoir-search.hurl