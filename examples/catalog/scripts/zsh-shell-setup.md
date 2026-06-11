# zsh

Install and configure ZSH as the default shell.

```bash
# Install zsh if it isn't already available
if ! command -v zsh &>/dev/null; then
  if command -v apt-get &>/dev/null; then
    sudo apt-get update && sudo apt-get install -y zsh
  elif command -v brew &>/dev/null; then
    brew install zsh
  else
    echo "Could not install zsh automatically. Please install zsh manually."
    exit 1
  fi
fi

# Install Oh My Zsh if it isn't already present
if [ ! -d "$HOME/.oh-my-zsh" ]; then
  echo "downloading and running Oh My Zsh install script ..."
  sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended
fi

# Install ZAP if it isn't already present
if [ ! -d "$HOME/.local/share/zap" ]; then
  echo "downloading and running ZAP install script ..."
  curl -s https://raw.githubusercontent.com/zap-zsh/zap/master/install.zsh > /tmp/zap-install.zsh
  zsh /tmp/zap-install.zsh --branch release-v1 --unattended
  rm -f /tmp/zap-install.zsh
fi

# Set zsh as the default shell if it isn't already
ZSH_PATH=$(command -v zsh)
if [ "$SHELL" != "$ZSH_PATH" ]; then
  echo "setting default shell to zsh ..."
  chsh -s "$ZSH_PATH"
fi

# Source zshrc to apply any new configuration changes
zsh -c "source ~/.zshrc"
```
