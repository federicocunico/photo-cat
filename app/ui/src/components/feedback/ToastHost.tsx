import { Toast } from "./Toast";
import { useToast } from "../../store/toasts";
import { BusyButton } from "./BusyButton";

export function ToastHost() {
  const { notices, dismiss } = useToast();
  return (
    <div className="feedback-toasts">
      {notices.map((n) => (
        <Toast key={n.id} stacked duration={n.duration} onClose={() => dismiss(n.id)}>
          {n.message}
          {n.action && (
            <BusyButton
              className="btn"
              onClick={async () => {
                await n.action!.run();
                dismiss(n.id);
              }}
            >
              {n.action.label}
            </BusyButton>
          )}
        </Toast>
      ))}
    </div>
  );
}
