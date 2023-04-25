import * as z from "zod";

export const TauriInvokeErr = z
  .object({
    StartErr: z.string().optional(),
    ResponseErr: z.string().optional(),
    AddEntryErr: z.string().optional(),
    ListEntriesErr: z.string().optional(),
    GetEntryErr: z.string().optional(),
    UnKnownErr: z.string().optional(),
  })
  .refine((data) => Object.values(data).some((v) => Boolean(v)));

export type TauriInvokeErr = z.infer<typeof TauriInvokeErr>;
