#!/bin/bash

set -e

# Function to get user input with a prompt
read_input() {
    read -p "$1: " input
    echo "$input"
}

# Paths
TIMER_FILE="/etc/systemd/system/cloudflare-ddns.timer"
BACKUP_FILE="/etc/systemd/system/cloudflare-ddns.timer.bak"

# Check if systemd timer exists
if [ ! -f "$TIMER_FILE" ]; then
    echo "❌ Error: Timer file $TIMER_FILE does not exist. Ensure that the service is properly installed first."
    exit 1
fi

# Print current timer settings
echo "✅ Found systemd timer: $TIMER_FILE"
echo "-----------------------------------------"
echo "Current OnCalendar value:"
grep "OnCalendar=" "$TIMER_FILE" || echo "No OnCalendar entry found."
echo "-----------------------------------------"

# Show detailed examples for user
echo
echo "🕑 Enter a new timer interval for how often the DDNS updater should run."
echo
echo "👉 Example formats for 'OnCalendar':"
echo "   - 'hourly'                  : Every hour"
echo "   - 'daily'                   : Every day at midnight"
echo "   - 'weekly'                  : Every Monday at midnight"
echo "   - 'monthly'                 : First day of each month at midnight"
echo "   - 'yearly'                  : January 1st each year at midnight"
echo "   - 'Mon *-*-* 03:00:00'      : Every Monday at 3:00 AM"
echo "   - '*-*-* 03:00:00'          : Every day at 3:00 AM"
echo "   - '*-*-1 00:00:00'          : First day of every month at midnight"
echo "   - '2025-03-01 00:00:00'     : Specific date and time (March 1, 2025 at midnight)"
echo "   - 'Mon,Fri 12:00'           : Every Monday and Friday at 12:00 PM"
echo "   - 'Mon *-*-* 08,12,16:00:00': Every Monday at 8 AM, 12 PM, and 4 PM"
echo
echo "ℹ️  More examples: https://www.freedesktop.org/software/systemd/man/systemd.time.html"
echo

# Function to validate basic format of OnCalendar input
validate_oncalendar() {
    local input="$1"

    # Basic regex to catch "obviously wrong" formats (not a full systemd parser)
    if [[ "$input" =~ ^([A-Za-z]{3}|\*|[0-9]{4})([ ,/-][0-9*]{1,2}){0,3}([ :][0-9*]{1,2}){0,3}$ ]] || [[ "$input" =~ ^(hourly|daily|weekly|monthly|yearly)$ ]]; then
        return 0
    else
        return 1
    fi
}

# Loop until valid input is received
while true; do
    new_interval=$(read_input "📝 Enter your desired 'OnCalendar' time format")
    if validate_oncalendar "$new_interval"; then
        echo "✅ Accepted time format: $new_interval"
        break
    else
        echo "❌ Invalid time format: '$new_interval'. Please try again following the given examples."
    fi
done

# Backup the existing timer file
cp "$TIMER_FILE" "$BACKUP_FILE"
echo "✅ Backup created at $BACKUP_FILE"

# Update OnCalendar line
if grep -q "OnCalendar=" "$TIMER_FILE"; then
    sed -i "s|OnCalendar=.*|OnCalendar=$new_interval|g" "$TIMER_FILE"
else
    # If no OnCalendar exists, add it under [Timer] section
    sed -i "/^\[Timer\]/a OnCalendar=$new_interval" "$TIMER_FILE"
fi

echo "✅ Updated OnCalendar to: $new_interval"

# Reload systemd daemon and restart the timer
systemctl daemon-reload
systemctl restart cloudflare-ddns.timer

echo "✅ Timer updated and restarted successfully!"

# Show timer status
echo "-----------------------------------------"
systemctl status cloudflare-ddns.timer --no-pager
echo "-----------------------------------------"
