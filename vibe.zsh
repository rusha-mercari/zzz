# Universal AI Assistant Invoker
v() {
    local assistant="$1"
    shift

    if ! op account get &>/dev/null; then
        echo "Not signed in to 1Password (example account). Signing in..." >&2
        local signin_output=$(op signin 2>/dev/null)
        if [[ $? -ne 0 || -z "$signin_output" ]]; then
            echo "Error: Failed to sign in to 1Password" >&2
            return 1
        fi
        eval "$signin_output"
    fi

    local api_key=$(op read "op://Employee/litellm/credential" 2>/dev/null)
    if [[ -z "$api_key" ]]; then
        echo "Error: Failed to retrieve API key from 1Password" >&2
        echo "Please check that the credential 'op://Employee/litellm/credential' exists and is accessible" >&2
        return 1
    fi
    local base_url="https://litellm.example.in"
    
    case "$assistant" in
        oco)
            OCO_API_KEY="$api_key" OCO_API_URL="$base_url" command oco "$@"
            ;;
        claude)
            ANTHROPIC_AUTH_TOKEN="$api_key" ANTHROPIC_BASE_URL="$base_url" command claude "$@"
            ;;
        aider)
            command aider --model openai/gpt-4o --openai-api-key "$api_key" --openai-api-base "$base_url/v1" "$@"
            ;;
        codex)
            OPENAI_BASE_URL="$base_url/v1" OPENAI_API_KEY="$api_key" codex --model o4-mini "$@"
            ;;
        *)
            echo "Unknown assistant: $assistant"
            echo "Available: oco, claude, aider, codex"
            return 1
            ;;
    esac
}

zzz() {
    local taskid="$1"
    local task_description="$2"
    local folder="$3"

    if [[ -z "$taskid" || -z "$task_description" || -z "$folder" ]]; then
        echo "Usage: zzz <taskid> <task_description> <folder>"
        return 1
    fi

    # Check if logged in to 1Password
    if ! op account get &>/dev/null; then
        echo "Not signed in to 1Password. Signing in..." >&2
        if ! eval $(op signin); then
            echo "Error: Failed to sign in to 1Password" >&2
            return 1
        fi
    fi

    # Fetch API key
    local api_key=$(op read "op://Employee/litellm/credential" 2>/dev/null)
    if [[ -z "$api_key" ]]; then
        echo "Error: Failed to retrieve API key from 1Password" >&2
        return 1
    fi

    local litellm_url="https://litellm.example.in"

    # Convert folder to absolute path
    local abs_folder
    abs_folder=$(realpath "$folder" 2>/dev/null)
    if [[ $? -ne 0 || ! -d "$abs_folder" ]]; then
        echo "Error: Directory '$folder' does not exist or cannot be resolved to absolute path"
        return 1
    fi

    local zellij_layout=$(cat <<EOF
layout {
  // Root
  pane split_direction="horizontal" {
    pane split_direction="vertical" {
        // Info
        pane split_direction="horizontal" {
        pane name="Task List" command="$EDITOR" {
            args ".zzz/task-${taskid}/todo-list.md"
        }
        pane name="Overseer"
        pane name="Review" command="$EDITOR" {
            args ".zzz/task-${taskid}/review.md"
        }
        }
        // Main
        pane split_direction="horizontal" size="70%" {
          pane size="60%" name="Editor" command="$EDITOR"
          pane size="40%" name="Commander"
        }
    }
    // ZZZ Plugin Status Bar
    pane size=1 borderless=true {
        plugin location="file:/Users/rusha/code/zellij/plugins/zzz/target/wasm32-wasip1/debug/zzz.wasm" {
            task_id "$taskid"
            task_description "$task_description"
            api_key "$api_key"
            litellm_url "$litellm_url"
        }
    }
  }
}
EOF
)
    local layout_file="/tmp/zellij_layout_${taskid}.kdl"
    echo "$zellij_layout" > "$layout_file"
    zellij action new-tab --layout "$layout_file" --cwd "$abs_folder"
    rm "$layout_file"
}
