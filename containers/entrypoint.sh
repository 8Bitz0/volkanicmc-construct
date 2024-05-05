#!/usr/bin/env bash

# Make sure the "USER" variable is set correctly
export USER=$(whoami)

# Ensure the user is on the "volkanic" user
if [[ $USER != "volkanic" ]]; then
    echo "This entrypoint script is only designed for the VolkanicMC Construct Docker container."
    exit 1
fi

if [[ $MIN_MEM == "" ]]; then
    echo "Please specify the minimum runtime memory usage in megabytes (use the\"MIN_MEM\" variable)"
    exit 1
elif [[ $MAX_MEM == "" ]]; then
    echo "Please specify the maximum runtime memory usage in megabytes (use the\"MAX_MEM\" variable)"
    exit 1
fi

if [[ $VK_TEMPLATE_BASE64 != "" ]]; then
    echo "Writing template from provided base64"
    if [ -f /etc/os-release ] && grep -q 'ID=alpine' /etc/os-release; then
        echo $(echo $VK_TEMPLATE_BASE64 | base64 -d) > template.json
    else
        echo $(echo $VK_TEMPLATE_BASE64 | base64 --decode) > template.json
    fi
elif [[ $VK_TEMPLATE_URL != "" ]]; then
    echo "Writing template from provided URL"
    wget -O template.json $VK_TEMPLATE_URL
else
    echo "No VolkanicMC Construct template provided. Use the \"VK_TEMPLATE_URL\" environment variable to provide a URL."
    exit 1
fi

export PATH="${PATH}:~/.local/bin"

if [[ -f ".volkanic/build.json" ]]; then
    echo "Build already present"
elif [[ $ALWAYS_REBUILD == "1" ]]; then
    volkanicmc-construct -b /server build --force --allow-custom-jvm-args -j="-Xms"$MIN_MEM"M -Xmx"$MAX_MEM"M" template.json
else
    volkanicmc-construct -b /server build --allow-custom-jvm-args -j="-Xms"$MIN_MEM"M -Xmx"$MAX_MEM"M" template.json
fi

# Create VolkanicMC start script
volkanicmc-construct -b /server exec-script > vkstart.sh
chmod +x vkstart.sh

# Execute the start script
exec ./vkstart.sh
