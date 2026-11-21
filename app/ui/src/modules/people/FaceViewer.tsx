// Visore di photo-cat sui volti: la foto con il riquadro del volto evidenziato.
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type FaceView } from "../../lib/ipc";
import { Viewer } from "../../components/viewer/Viewer";
import { thumbUrl } from "../library/thumbUrl";

export function FaceViewer({
  faces,
  index,
  onIndex,
  onClose,
}: {
  faces: FaceView[];
  index: number;
  onIndex: (n: number) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const face = faces[index];
  const detail = useQuery({
    queryKey: ["media-detail", face?.mediaId],
    queryFn: () => commands.mediaDetail(face!.mediaId),
    enabled: isTauri() && face != null,
  });
  return (
    <Viewer
      index={index}
      total={faces.length}
      photo={
        face
          ? {
              id: face.mediaId,
              name: detail.data?.item.name ?? t("viewer.loading"),
              src: thumbUrl(face.thumb),
              highlight: face.bbox,
            }
          : null
      }
      onIndex={onIndex}
      onClose={onClose}
    />
  );
}
