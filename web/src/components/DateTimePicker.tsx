interface DateTimePickerProps {
  label: string;
  date: string;
  hour: number;
  minute: number;
  required?: boolean;
  onDateChange: (value: string) => void;
  onHourChange: (value: number) => void;
  onMinuteChange: (value: number) => void;
}

const inputClass =
  "w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm dark:border-gray-700 dark:bg-gray-800";

const hours = Array.from({ length: 24 }, (_, i) => i);
const minutes = Array.from({ length: 60 }, (_, i) => i);

export default function DateTimePicker({
  label,
  date,
  hour,
  minute,
  required,
  onDateChange,
  onHourChange,
  onMinuteChange,
}: DateTimePickerProps) {
  return (
    <div>
      <span className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-200">
        {label}
        {required ? " *" : ""}
      </span>
      <div className="grid grid-cols-[1fr_4.5rem_4.5rem] gap-2">
        <input
          type="date"
          value={date}
          onChange={(e) => onDateChange(e.target.value)}
          required={required}
          className={inputClass}
        />
        <select
          value={hour}
          onChange={(e) => onHourChange(parseInt(e.target.value, 10))}
          className={inputClass}
          aria-label={`${label}小时`}
        >
          {hours.map((h) => (
            <option key={h} value={h}>
              {String(h).padStart(2, "0")} 时
            </option>
          ))}
        </select>
        <select
          value={minute}
          onChange={(e) => onMinuteChange(parseInt(e.target.value, 10))}
          className={inputClass}
          aria-label={`${label}分钟`}
        >
          {minutes.map((m) => (
            <option key={m} value={m}>
              {String(m).padStart(2, "0")} 分
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
